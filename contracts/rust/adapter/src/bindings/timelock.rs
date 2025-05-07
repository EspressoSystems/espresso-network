///Module containing a contract's types and functions.
/**

```solidity
library TimelockController {
    type OperationState is uint8;
}
```*/
#[allow(
    non_camel_case_types,
    non_snake_case,
    clippy::pub_underscore_fields,
    clippy::style,
    clippy::empty_structs_with_brackets
)]
pub mod TimelockController {
    use alloy::sol_types as alloy_sol_types;

    use super::*;
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct OperationState(u8);
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[automatically_derived]
        impl alloy_sol_types::private::SolTypeValue<OperationState> for u8 {
            #[inline]
            fn stv_to_tokens(
                &self,
            ) -> <alloy::sol_types::sol_data::Uint<8> as alloy_sol_types::SolType>::Token<'_>
            {
                alloy_sol_types::private::SolTypeValue::<
                    alloy::sol_types::sol_data::Uint<8>,
                >::stv_to_tokens(self)
            }
            #[inline]
            fn stv_eip712_data_word(&self) -> alloy_sol_types::Word {
                <alloy::sol_types::sol_data::Uint<8> as alloy_sol_types::SolType>::tokenize(self).0
            }
            #[inline]
            fn stv_abi_encode_packed_to(&self, out: &mut alloy_sol_types::private::Vec<u8>) {
                <alloy::sol_types::sol_data::Uint<
                    8,
                > as alloy_sol_types::SolType>::abi_encode_packed_to(self, out)
            }
            #[inline]
            fn stv_abi_packed_encoded_size(&self) -> usize {
                <alloy::sol_types::sol_data::Uint<8> as alloy_sol_types::SolType>::abi_encoded_size(
                    self,
                )
            }
        }
        #[automatically_derived]
        impl OperationState {
            /// The Solidity type name.
            pub const NAME: &'static str = stringify!(@ name);
            /// Convert from the underlying value type.
            #[inline]
            pub const fn from(value: u8) -> Self {
                Self(value)
            }
            /// Return the underlying value.
            #[inline]
            pub const fn into(self) -> u8 {
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
        impl alloy_sol_types::SolType for OperationState {
            type RustType = u8;
            type Token<'a> =
                <alloy::sol_types::sol_data::Uint<8> as alloy_sol_types::SolType>::Token<'a>;
            const SOL_NAME: &'static str = Self::NAME;
            const ENCODED_SIZE: Option<usize> =
                <alloy::sol_types::sol_data::Uint<8> as alloy_sol_types::SolType>::ENCODED_SIZE;
            const PACKED_ENCODED_SIZE: Option<usize> = <alloy::sol_types::sol_data::Uint<
                8,
            > as alloy_sol_types::SolType>::PACKED_ENCODED_SIZE;
            #[inline]
            fn valid_token(token: &Self::Token<'_>) -> bool {
                Self::type_check(token).is_ok()
            }
            #[inline]
            fn type_check(token: &Self::Token<'_>) -> alloy_sol_types::Result<()> {
                <alloy::sol_types::sol_data::Uint<8> as alloy_sol_types::SolType>::type_check(token)
            }
            #[inline]
            fn detokenize(token: Self::Token<'_>) -> Self::RustType {
                <alloy::sol_types::sol_data::Uint<8> as alloy_sol_types::SolType>::detokenize(token)
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::EventTopic for OperationState {
            #[inline]
            fn topic_preimage_length(rust: &Self::RustType) -> usize {
                <alloy::sol_types::sol_data::Uint<
                    8,
                > as alloy_sol_types::EventTopic>::topic_preimage_length(rust)
            }
            #[inline]
            fn encode_topic_preimage(
                rust: &Self::RustType,
                out: &mut alloy_sol_types::private::Vec<u8>,
            ) {
                <alloy::sol_types::sol_data::Uint<
                    8,
                > as alloy_sol_types::EventTopic>::encode_topic_preimage(rust, out)
            }
            #[inline]
            fn encode_topic(rust: &Self::RustType) -> alloy_sol_types::abi::token::WordToken {
                <alloy::sol_types::sol_data::Uint<8> as alloy_sol_types::EventTopic>::encode_topic(
                    rust,
                )
            }
        }
    };
    use alloy::contract as alloy_contract;
    /**Creates a new wrapper around an on-chain [`TimelockController`](self) contract instance.

    See the [wrapper's documentation](`TimelockControllerInstance`) for more details.*/
    #[inline]
    pub const fn new<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    >(
        address: alloy_sol_types::private::Address,
        provider: P,
    ) -> TimelockControllerInstance<T, P, N> {
        TimelockControllerInstance::<T, P, N>::new(address, provider)
    }
    /**A [`TimelockController`](self) instance.

    Contains type-safe methods for interacting with an on-chain instance of the
    [`TimelockController`](self) contract located at a given `address`, using a given
    provider `P`.

    If the contract bytecode is available (see the [`sol!`](alloy_sol_types::sol!)
    documentation on how to provide it), the `deploy` and `deploy_builder` methods can
    be used to deploy a new instance of the contract.

    See the [module-level documentation](self) for all the available methods.*/
    #[derive(Clone)]
    pub struct TimelockControllerInstance<T, P, N = alloy_contract::private::Ethereum> {
        address: alloy_sol_types::private::Address,
        provider: P,
        _network_transport: ::core::marker::PhantomData<(N, T)>,
    }
    #[automatically_derived]
    impl<T, P, N> ::core::fmt::Debug for TimelockControllerInstance<T, P, N> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple("TimelockControllerInstance")
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
        > TimelockControllerInstance<T, P, N>
    {
        /**Creates a new wrapper around an on-chain [`TimelockController`](self) contract instance.

        See the [wrapper's documentation](`TimelockControllerInstance`) for more details.*/
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
    impl<T, P: ::core::clone::Clone, N> TimelockControllerInstance<T, &P, N> {
        /// Clones the provider and returns a new instance with the cloned provider.
        #[inline]
        pub fn with_cloned_provider(self) -> TimelockControllerInstance<T, P, N> {
            TimelockControllerInstance {
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
        > TimelockControllerInstance<T, P, N>
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
        > TimelockControllerInstance<T, P, N>
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
library TimelockController {
    type OperationState is uint8;
}

interface Timelock {
    error AccessControlBadConfirmation();
    error AccessControlUnauthorizedAccount(address account, bytes32 neededRole);
    error FailedInnerCall();
    error TimelockInsufficientDelay(uint256 delay, uint256 minDelay);
    error TimelockInvalidOperationLength(uint256 targets, uint256 payloads, uint256 values);
    error TimelockUnauthorizedCaller(address caller);
    error TimelockUnexecutedPredecessor(bytes32 predecessorId);
    error TimelockUnexpectedOperationState(bytes32 operationId, bytes32 expectedStates);

    event CallExecuted(bytes32 indexed id, uint256 indexed index, address target, uint256 value, bytes data);
    event CallSalt(bytes32 indexed id, bytes32 salt);
    event CallScheduled(bytes32 indexed id, uint256 indexed index, address target, uint256 value, bytes data, bytes32 predecessor, uint256 delay);
    event Cancelled(bytes32 indexed id);
    event MinDelayChange(uint256 oldDuration, uint256 newDuration);
    event RoleAdminChanged(bytes32 indexed role, bytes32 indexed previousAdminRole, bytes32 indexed newAdminRole);
    event RoleGranted(bytes32 indexed role, address indexed account, address indexed sender);
    event RoleRevoked(bytes32 indexed role, address indexed account, address indexed sender);

    constructor(uint256 minDelay, address[] proposers, address[] executors, address admin);

    receive() external payable;

    function CANCELLER_ROLE() external view returns (bytes32);
    function DEFAULT_ADMIN_ROLE() external view returns (bytes32);
    function EXECUTOR_ROLE() external view returns (bytes32);
    function PROPOSER_ROLE() external view returns (bytes32);
    function cancel(bytes32 id) external;
    function execute(address target, uint256 value, bytes memory payload, bytes32 predecessor, bytes32 salt) external payable;
    function executeBatch(address[] memory targets, uint256[] memory values, bytes[] memory payloads, bytes32 predecessor, bytes32 salt) external payable;
    function getMinDelay() external view returns (uint256);
    function getOperationState(bytes32 id) external view returns (TimelockController.OperationState);
    function getRoleAdmin(bytes32 role) external view returns (bytes32);
    function getTimestamp(bytes32 id) external view returns (uint256);
    function grantRole(bytes32 role, address account) external;
    function hasRole(bytes32 role, address account) external view returns (bool);
    function hashOperation(address target, uint256 value, bytes memory data, bytes32 predecessor, bytes32 salt) external pure returns (bytes32);
    function hashOperationBatch(address[] memory targets, uint256[] memory values, bytes[] memory payloads, bytes32 predecessor, bytes32 salt) external pure returns (bytes32);
    function isOperation(bytes32 id) external view returns (bool);
    function isOperationDone(bytes32 id) external view returns (bool);
    function isOperationPending(bytes32 id) external view returns (bool);
    function isOperationReady(bytes32 id) external view returns (bool);
    function onERC1155BatchReceived(address, address, uint256[] memory, uint256[] memory, bytes memory) external returns (bytes4);
    function onERC1155Received(address, address, uint256, uint256, bytes memory) external returns (bytes4);
    function onERC721Received(address, address, uint256, bytes memory) external returns (bytes4);
    function renounceRole(bytes32 role, address callerConfirmation) external;
    function revokeRole(bytes32 role, address account) external;
    function schedule(address target, uint256 value, bytes memory data, bytes32 predecessor, bytes32 salt, uint256 delay) external;
    function scheduleBatch(address[] memory targets, uint256[] memory values, bytes[] memory payloads, bytes32 predecessor, bytes32 salt, uint256 delay) external;
    function supportsInterface(bytes4 interfaceId) external view returns (bool);
    function updateDelay(uint256 newDelay) external;
}
```

...which was generated by the following JSON ABI:
```json
[
  {
    "type": "constructor",
    "inputs": [
      {
        "name": "minDelay",
        "type": "uint256",
        "internalType": "uint256"
      },
      {
        "name": "proposers",
        "type": "address[]",
        "internalType": "address[]"
      },
      {
        "name": "executors",
        "type": "address[]",
        "internalType": "address[]"
      },
      {
        "name": "admin",
        "type": "address",
        "internalType": "address"
      }
    ],
    "stateMutability": "nonpayable"
  },
  {
    "type": "receive",
    "stateMutability": "payable"
  },
  {
    "type": "function",
    "name": "CANCELLER_ROLE",
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
    "name": "DEFAULT_ADMIN_ROLE",
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
    "name": "EXECUTOR_ROLE",
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
    "name": "PROPOSER_ROLE",
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
    "name": "cancel",
    "inputs": [
      {
        "name": "id",
        "type": "bytes32",
        "internalType": "bytes32"
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "execute",
    "inputs": [
      {
        "name": "target",
        "type": "address",
        "internalType": "address"
      },
      {
        "name": "value",
        "type": "uint256",
        "internalType": "uint256"
      },
      {
        "name": "payload",
        "type": "bytes",
        "internalType": "bytes"
      },
      {
        "name": "predecessor",
        "type": "bytes32",
        "internalType": "bytes32"
      },
      {
        "name": "salt",
        "type": "bytes32",
        "internalType": "bytes32"
      }
    ],
    "outputs": [],
    "stateMutability": "payable"
  },
  {
    "type": "function",
    "name": "executeBatch",
    "inputs": [
      {
        "name": "targets",
        "type": "address[]",
        "internalType": "address[]"
      },
      {
        "name": "values",
        "type": "uint256[]",
        "internalType": "uint256[]"
      },
      {
        "name": "payloads",
        "type": "bytes[]",
        "internalType": "bytes[]"
      },
      {
        "name": "predecessor",
        "type": "bytes32",
        "internalType": "bytes32"
      },
      {
        "name": "salt",
        "type": "bytes32",
        "internalType": "bytes32"
      }
    ],
    "outputs": [],
    "stateMutability": "payable"
  },
  {
    "type": "function",
    "name": "getMinDelay",
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
    "name": "getOperationState",
    "inputs": [
      {
        "name": "id",
        "type": "bytes32",
        "internalType": "bytes32"
      }
    ],
    "outputs": [
      {
        "name": "",
        "type": "uint8",
        "internalType": "enum TimelockController.OperationState"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "getRoleAdmin",
    "inputs": [
      {
        "name": "role",
        "type": "bytes32",
        "internalType": "bytes32"
      }
    ],
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
    "name": "getTimestamp",
    "inputs": [
      {
        "name": "id",
        "type": "bytes32",
        "internalType": "bytes32"
      }
    ],
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
    "name": "grantRole",
    "inputs": [
      {
        "name": "role",
        "type": "bytes32",
        "internalType": "bytes32"
      },
      {
        "name": "account",
        "type": "address",
        "internalType": "address"
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "hasRole",
    "inputs": [
      {
        "name": "role",
        "type": "bytes32",
        "internalType": "bytes32"
      },
      {
        "name": "account",
        "type": "address",
        "internalType": "address"
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
    "name": "hashOperation",
    "inputs": [
      {
        "name": "target",
        "type": "address",
        "internalType": "address"
      },
      {
        "name": "value",
        "type": "uint256",
        "internalType": "uint256"
      },
      {
        "name": "data",
        "type": "bytes",
        "internalType": "bytes"
      },
      {
        "name": "predecessor",
        "type": "bytes32",
        "internalType": "bytes32"
      },
      {
        "name": "salt",
        "type": "bytes32",
        "internalType": "bytes32"
      }
    ],
    "outputs": [
      {
        "name": "",
        "type": "bytes32",
        "internalType": "bytes32"
      }
    ],
    "stateMutability": "pure"
  },
  {
    "type": "function",
    "name": "hashOperationBatch",
    "inputs": [
      {
        "name": "targets",
        "type": "address[]",
        "internalType": "address[]"
      },
      {
        "name": "values",
        "type": "uint256[]",
        "internalType": "uint256[]"
      },
      {
        "name": "payloads",
        "type": "bytes[]",
        "internalType": "bytes[]"
      },
      {
        "name": "predecessor",
        "type": "bytes32",
        "internalType": "bytes32"
      },
      {
        "name": "salt",
        "type": "bytes32",
        "internalType": "bytes32"
      }
    ],
    "outputs": [
      {
        "name": "",
        "type": "bytes32",
        "internalType": "bytes32"
      }
    ],
    "stateMutability": "pure"
  },
  {
    "type": "function",
    "name": "isOperation",
    "inputs": [
      {
        "name": "id",
        "type": "bytes32",
        "internalType": "bytes32"
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
    "name": "isOperationDone",
    "inputs": [
      {
        "name": "id",
        "type": "bytes32",
        "internalType": "bytes32"
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
    "name": "isOperationPending",
    "inputs": [
      {
        "name": "id",
        "type": "bytes32",
        "internalType": "bytes32"
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
    "name": "isOperationReady",
    "inputs": [
      {
        "name": "id",
        "type": "bytes32",
        "internalType": "bytes32"
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
    "name": "onERC1155BatchReceived",
    "inputs": [
      {
        "name": "",
        "type": "address",
        "internalType": "address"
      },
      {
        "name": "",
        "type": "address",
        "internalType": "address"
      },
      {
        "name": "",
        "type": "uint256[]",
        "internalType": "uint256[]"
      },
      {
        "name": "",
        "type": "uint256[]",
        "internalType": "uint256[]"
      },
      {
        "name": "",
        "type": "bytes",
        "internalType": "bytes"
      }
    ],
    "outputs": [
      {
        "name": "",
        "type": "bytes4",
        "internalType": "bytes4"
      }
    ],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "onERC1155Received",
    "inputs": [
      {
        "name": "",
        "type": "address",
        "internalType": "address"
      },
      {
        "name": "",
        "type": "address",
        "internalType": "address"
      },
      {
        "name": "",
        "type": "uint256",
        "internalType": "uint256"
      },
      {
        "name": "",
        "type": "uint256",
        "internalType": "uint256"
      },
      {
        "name": "",
        "type": "bytes",
        "internalType": "bytes"
      }
    ],
    "outputs": [
      {
        "name": "",
        "type": "bytes4",
        "internalType": "bytes4"
      }
    ],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "onERC721Received",
    "inputs": [
      {
        "name": "",
        "type": "address",
        "internalType": "address"
      },
      {
        "name": "",
        "type": "address",
        "internalType": "address"
      },
      {
        "name": "",
        "type": "uint256",
        "internalType": "uint256"
      },
      {
        "name": "",
        "type": "bytes",
        "internalType": "bytes"
      }
    ],
    "outputs": [
      {
        "name": "",
        "type": "bytes4",
        "internalType": "bytes4"
      }
    ],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "renounceRole",
    "inputs": [
      {
        "name": "role",
        "type": "bytes32",
        "internalType": "bytes32"
      },
      {
        "name": "callerConfirmation",
        "type": "address",
        "internalType": "address"
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "revokeRole",
    "inputs": [
      {
        "name": "role",
        "type": "bytes32",
        "internalType": "bytes32"
      },
      {
        "name": "account",
        "type": "address",
        "internalType": "address"
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "schedule",
    "inputs": [
      {
        "name": "target",
        "type": "address",
        "internalType": "address"
      },
      {
        "name": "value",
        "type": "uint256",
        "internalType": "uint256"
      },
      {
        "name": "data",
        "type": "bytes",
        "internalType": "bytes"
      },
      {
        "name": "predecessor",
        "type": "bytes32",
        "internalType": "bytes32"
      },
      {
        "name": "salt",
        "type": "bytes32",
        "internalType": "bytes32"
      },
      {
        "name": "delay",
        "type": "uint256",
        "internalType": "uint256"
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "scheduleBatch",
    "inputs": [
      {
        "name": "targets",
        "type": "address[]",
        "internalType": "address[]"
      },
      {
        "name": "values",
        "type": "uint256[]",
        "internalType": "uint256[]"
      },
      {
        "name": "payloads",
        "type": "bytes[]",
        "internalType": "bytes[]"
      },
      {
        "name": "predecessor",
        "type": "bytes32",
        "internalType": "bytes32"
      },
      {
        "name": "salt",
        "type": "bytes32",
        "internalType": "bytes32"
      },
      {
        "name": "delay",
        "type": "uint256",
        "internalType": "uint256"
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "supportsInterface",
    "inputs": [
      {
        "name": "interfaceId",
        "type": "bytes4",
        "internalType": "bytes4"
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
    "name": "updateDelay",
    "inputs": [
      {
        "name": "newDelay",
        "type": "uint256",
        "internalType": "uint256"
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "event",
    "name": "CallExecuted",
    "inputs": [
      {
        "name": "id",
        "type": "bytes32",
        "indexed": true,
        "internalType": "bytes32"
      },
      {
        "name": "index",
        "type": "uint256",
        "indexed": true,
        "internalType": "uint256"
      },
      {
        "name": "target",
        "type": "address",
        "indexed": false,
        "internalType": "address"
      },
      {
        "name": "value",
        "type": "uint256",
        "indexed": false,
        "internalType": "uint256"
      },
      {
        "name": "data",
        "type": "bytes",
        "indexed": false,
        "internalType": "bytes"
      }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "CallSalt",
    "inputs": [
      {
        "name": "id",
        "type": "bytes32",
        "indexed": true,
        "internalType": "bytes32"
      },
      {
        "name": "salt",
        "type": "bytes32",
        "indexed": false,
        "internalType": "bytes32"
      }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "CallScheduled",
    "inputs": [
      {
        "name": "id",
        "type": "bytes32",
        "indexed": true,
        "internalType": "bytes32"
      },
      {
        "name": "index",
        "type": "uint256",
        "indexed": true,
        "internalType": "uint256"
      },
      {
        "name": "target",
        "type": "address",
        "indexed": false,
        "internalType": "address"
      },
      {
        "name": "value",
        "type": "uint256",
        "indexed": false,
        "internalType": "uint256"
      },
      {
        "name": "data",
        "type": "bytes",
        "indexed": false,
        "internalType": "bytes"
      },
      {
        "name": "predecessor",
        "type": "bytes32",
        "indexed": false,
        "internalType": "bytes32"
      },
      {
        "name": "delay",
        "type": "uint256",
        "indexed": false,
        "internalType": "uint256"
      }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "Cancelled",
    "inputs": [
      {
        "name": "id",
        "type": "bytes32",
        "indexed": true,
        "internalType": "bytes32"
      }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "MinDelayChange",
    "inputs": [
      {
        "name": "oldDuration",
        "type": "uint256",
        "indexed": false,
        "internalType": "uint256"
      },
      {
        "name": "newDuration",
        "type": "uint256",
        "indexed": false,
        "internalType": "uint256"
      }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "RoleAdminChanged",
    "inputs": [
      {
        "name": "role",
        "type": "bytes32",
        "indexed": true,
        "internalType": "bytes32"
      },
      {
        "name": "previousAdminRole",
        "type": "bytes32",
        "indexed": true,
        "internalType": "bytes32"
      },
      {
        "name": "newAdminRole",
        "type": "bytes32",
        "indexed": true,
        "internalType": "bytes32"
      }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "RoleGranted",
    "inputs": [
      {
        "name": "role",
        "type": "bytes32",
        "indexed": true,
        "internalType": "bytes32"
      },
      {
        "name": "account",
        "type": "address",
        "indexed": true,
        "internalType": "address"
      },
      {
        "name": "sender",
        "type": "address",
        "indexed": true,
        "internalType": "address"
      }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "RoleRevoked",
    "inputs": [
      {
        "name": "role",
        "type": "bytes32",
        "indexed": true,
        "internalType": "bytes32"
      },
      {
        "name": "account",
        "type": "address",
        "indexed": true,
        "internalType": "address"
      },
      {
        "name": "sender",
        "type": "address",
        "indexed": true,
        "internalType": "address"
      }
    ],
    "anonymous": false
  },
  {
    "type": "error",
    "name": "AccessControlBadConfirmation",
    "inputs": []
  },
  {
    "type": "error",
    "name": "AccessControlUnauthorizedAccount",
    "inputs": [
      {
        "name": "account",
        "type": "address",
        "internalType": "address"
      },
      {
        "name": "neededRole",
        "type": "bytes32",
        "internalType": "bytes32"
      }
    ]
  },
  {
    "type": "error",
    "name": "FailedInnerCall",
    "inputs": []
  },
  {
    "type": "error",
    "name": "TimelockInsufficientDelay",
    "inputs": [
      {
        "name": "delay",
        "type": "uint256",
        "internalType": "uint256"
      },
      {
        "name": "minDelay",
        "type": "uint256",
        "internalType": "uint256"
      }
    ]
  },
  {
    "type": "error",
    "name": "TimelockInvalidOperationLength",
    "inputs": [
      {
        "name": "targets",
        "type": "uint256",
        "internalType": "uint256"
      },
      {
        "name": "payloads",
        "type": "uint256",
        "internalType": "uint256"
      },
      {
        "name": "values",
        "type": "uint256",
        "internalType": "uint256"
      }
    ]
  },
  {
    "type": "error",
    "name": "TimelockUnauthorizedCaller",
    "inputs": [
      {
        "name": "caller",
        "type": "address",
        "internalType": "address"
      }
    ]
  },
  {
    "type": "error",
    "name": "TimelockUnexecutedPredecessor",
    "inputs": [
      {
        "name": "predecessorId",
        "type": "bytes32",
        "internalType": "bytes32"
      }
    ]
  },
  {
    "type": "error",
    "name": "TimelockUnexpectedOperationState",
    "inputs": [
      {
        "name": "operationId",
        "type": "bytes32",
        "internalType": "bytes32"
      },
      {
        "name": "expectedStates",
        "type": "bytes32",
        "internalType": "bytes32"
      }
    ]
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
pub mod Timelock {
    use alloy::sol_types as alloy_sol_types;

    use super::*;
    /// The creation / init bytecode of the contract.
    ///
    /// ```text
    ///0x608060405234801561000f575f5ffd5b50604051611d0b380380611d0b83398101604081905261002e916102fe565b8383838361003c5f30610183565b506001600160a01b03811615610058576100565f82610183565b505b5f5b83518110156100ec576100ac7fb09aa5aeb3702cfd50b6b62bc4532604938f21248a27a1d5ca736082b6819cc18583815181106100995761009961037d565b602002602001015161018360201b60201c565b506100e37ffd643c72710c63c0180259aba6b2d05451e3591a24e58b62239378085726f7838583815181106100995761009961037d565b5060010161005a565b505f5b82518110156101375761012e7fd8aa0f3194971a2a116679f7c2090f6939c8d4e01a2a8d7e41d55e5351469e638483815181106100995761009961037d565b506001016100ef565b506002849055604080515f8152602081018690527f11c24f4ead16507c69ac467fbd5e4eed5fb5c699626d2cc6d66421df253886d5910160405180910390a15050505050505050610391565b5f828152602081815260408083206001600160a01b038516845290915281205460ff16610223575f838152602081815260408083206001600160a01b03861684529091529020805460ff191660011790556101db3390565b6001600160a01b0316826001600160a01b0316847f2f8788117e7eff1d82e926ec794901d17c78024a50270940304540a733656f0d60405160405180910390a4506001610226565b505f5b92915050565b634e487b7160e01b5f52604160045260245ffd5b80516001600160a01b0381168114610256575f5ffd5b919050565b5f82601f83011261026a575f5ffd5b81516001600160401b038111156102835761028361022c565b604051600582901b90603f8201601f191681016001600160401b03811182821017156102b1576102b161022c565b6040529182526020818501810192908101868411156102ce575f5ffd5b6020860192505b838310156102f4576102e683610240565b8152602092830192016102d5565b5095945050505050565b5f5f5f5f60808587031215610311575f5ffd5b845160208601519094506001600160401b0381111561032e575f5ffd5b61033a8782880161025b565b604087015190945090506001600160401b03811115610357575f5ffd5b6103638782880161025b565b92505061037260608601610240565b905092959194509250565b634e487b7160e01b5f52603260045260245ffd5b61196d8061039e5f395ff3fe6080604052600436106101b2575f3560e01c80638065657f116100e7578063bc197c8111610087578063d547741f11610062578063d547741f14610546578063e38335e514610565578063f23a6e6114610578578063f27a0c92146105a3575f5ffd5b8063bc197c81146104d1578063c4d252f5146104fc578063d45c44351461051b575f5ffd5b806391d14854116100c257806391d148541461044d578063a217fddf1461046c578063b08e51c01461047f578063b1c5f427146104b2575f5ffd5b80638065657f146103dc5780638f2a0bb0146103fb5780638f61f4f51461041a575f5ffd5b80632ab0f5291161015257806336568abe1161012d57806336568abe14610353578063584b153e1461037257806364d62353146103915780637958004c146103b0575f5ffd5b80632ab0f529146102f65780632f2ff15d1461031557806331d5075014610334575f5ffd5b8063134008d31161018d578063134008d31461025357806313bc9f2014610266578063150b7a0214610285578063248a9ca3146102c8575f5ffd5b806301d5062a146101bd57806301ffc9a7146101de57806307bd026514610212575f5ffd5b366101b957005b5f5ffd5b3480156101c8575f5ffd5b506101dc6101d7366004611163565b6105b7565b005b3480156101e9575f5ffd5b506101fd6101f83660046111d1565b61068b565b60405190151581526020015b60405180910390f35b34801561021d575f5ffd5b506102457fd8aa0f3194971a2a116679f7c2090f6939c8d4e01a2a8d7e41d55e5351469e6381565b604051908152602001610209565b6101dc6102613660046111f8565b61069b565b348015610271575f5ffd5b506101fd61028036600461125e565b61074d565b348015610290575f5ffd5b506102af61029f366004611324565b630a85bd0160e11b949350505050565b6040516001600160e01b03199091168152602001610209565b3480156102d3575f5ffd5b506102456102e236600461125e565b5f9081526020819052604090206001015490565b348015610301575f5ffd5b506101fd61031036600461125e565b610772565b348015610320575f5ffd5b506101dc61032f366004611387565b61077a565b34801561033f575f5ffd5b506101fd61034e36600461125e565b6107a4565b34801561035e575f5ffd5b506101dc61036d366004611387565b6107c8565b34801561037d575f5ffd5b506101fd61038c36600461125e565b610800565b34801561039c575f5ffd5b506101dc6103ab36600461125e565b610845565b3480156103bb575f5ffd5b506103cf6103ca36600461125e565b6108b8565b60405161020991906113c5565b3480156103e7575f5ffd5b506102456103f63660046111f8565b610900565b348015610406575f5ffd5b506101dc61041536600461142b565b61093e565b348015610425575f5ffd5b506102457fb09aa5aeb3702cfd50b6b62bc4532604938f21248a27a1d5ca736082b6819cc181565b348015610458575f5ffd5b506101fd610467366004611387565b610aca565b348015610477575f5ffd5b506102455f81565b34801561048a575f5ffd5b506102457ffd643c72710c63c0180259aba6b2d05451e3591a24e58b62239378085726f78381565b3480156104bd575f5ffd5b506102456104cc3660046114dd565b610af2565b3480156104dc575f5ffd5b506102af6104eb366004611606565b63bc197c8160e01b95945050505050565b348015610507575f5ffd5b506101dc61051636600461125e565b610b36565b348015610526575f5ffd5b5061024561053536600461125e565b5f9081526001602052604090205490565b348015610551575f5ffd5b506101dc610560366004611387565b610be0565b6101dc6105733660046114dd565b610c04565b348015610583575f5ffd5b506102af6105923660046116b2565b63f23a6e6160e01b95945050505050565b3480156105ae575f5ffd5b50600254610245565b7fb09aa5aeb3702cfd50b6b62bc4532604938f21248a27a1d5ca736082b6819cc16105e181610d85565b5f6105f0898989898989610900565b90506105fc8184610d92565b5f817f4cf4410cc57040e44862ef0f45f3dd5a5e02db8eb8add648d4b0e236f1d07dca8b8b8b8b8b8a6040516106379695949392919061172d565b60405180910390a3831561068057807f20fda5fd27a1ea7bf5b9567f143ac5470bb059374a27e8f67cb44f946f6d03878560405161067791815260200190565b60405180910390a25b505050505050505050565b5f61069582610e23565b92915050565b7fd8aa0f3194971a2a116679f7c2090f6939c8d4e01a2a8d7e41d55e5351469e636106c6815f610aca565b6106d4576106d48133610e47565b5f6106e3888888888888610900565b90506106ef8185610e84565b6106fb88888888610ed2565b5f817fc2617efa69bab66782fa219543714338489c4e9e178271560a91b82c3f612b588a8a8a8a6040516107329493929190611769565b60405180910390a361074381610f46565b5050505050505050565b5f60025b61075a836108b8565b600381111561076b5761076b6113b1565b1492915050565b5f6003610751565b5f8281526020819052604090206001015461079481610d85565b61079e8383610f71565b50505050565b5f806107af836108b8565b60038111156107c0576107c06113b1565b141592915050565b6001600160a01b03811633146107f15760405163334bd91960e11b815260040160405180910390fd5b6107fb8282611000565b505050565b5f5f61080b836108b8565b90506001816003811115610821576108216113b1565b148061083e5750600281600381111561083c5761083c6113b1565b145b9392505050565b333081146108765760405163e2850c5960e01b81526001600160a01b03821660048201526024015b60405180910390fd5b60025460408051918252602082018490527f11c24f4ead16507c69ac467fbd5e4eed5fb5c699626d2cc6d66421df253886d5910160405180910390a150600255565b5f81815260016020526040812054805f036108d557505f92915050565b600181036108e65750600392915050565b428111156108f75750600192915050565b50600292915050565b5f86868686868660405160200161091c9695949392919061172d565b6040516020818303038152906040528051906020012090509695505050505050565b7fb09aa5aeb3702cfd50b6b62bc4532604938f21248a27a1d5ca736082b6819cc161096881610d85565b88871415806109775750888514155b156109a9576040516001624fcdef60e01b03198152600481018a9052602481018690526044810188905260640161086d565b5f6109ba8b8b8b8b8b8b8b8b610af2565b90506109c68184610d92565b5f5b8a811015610a7b5780827f4cf4410cc57040e44862ef0f45f3dd5a5e02db8eb8add648d4b0e236f1d07dca8e8e85818110610a0557610a05611790565b9050602002016020810190610a1a91906117a4565b8d8d86818110610a2c57610a2c611790565b905060200201358c8c87818110610a4557610a45611790565b9050602002810190610a5791906117bd565b8c8b604051610a6b9695949392919061172d565b60405180910390a36001016109c8565b508315610abd57807f20fda5fd27a1ea7bf5b9567f143ac5470bb059374a27e8f67cb44f946f6d038785604051610ab491815260200190565b60405180910390a25b5050505050505050505050565b5f918252602082815260408084206001600160a01b0393909316845291905290205460ff1690565b5f8888888888888888604051602001610b12989796959493929190611893565b60405160208183030381529060405280519060200120905098975050505050505050565b7ffd643c72710c63c0180259aba6b2d05451e3591a24e58b62239378085726f783610b6081610d85565b610b6982610800565b610ba55781610b786002611069565b610b826001611069565b604051635ead8eb560e01b8152600481019390935217602482015260440161086d565b5f828152600160205260408082208290555183917fbaa1eb22f2a492ba1a5fea61b8df4d27c6c8b5f3971e63bb58fa14ff72eedb7091a25050565b5f82815260208190526040902060010154610bfa81610d85565b61079e8383611000565b7fd8aa0f3194971a2a116679f7c2090f6939c8d4e01a2a8d7e41d55e5351469e63610c2f815f610aca565b610c3d57610c3d8133610e47565b8786141580610c4c5750878414155b15610c7e576040516001624fcdef60e01b0319815260048101899052602481018590526044810187905260640161086d565b5f610c8f8a8a8a8a8a8a8a8a610af2565b9050610c9b8185610e84565b5f5b89811015610d6f575f8b8b83818110610cb857610cb8611790565b9050602002016020810190610ccd91906117a4565b90505f8a8a84818110610ce257610ce2611790565b905060200201359050365f8a8a86818110610cff57610cff611790565b9050602002810190610d1191906117bd565b91509150610d2184848484610ed2565b84867fc2617efa69bab66782fa219543714338489c4e9e178271560a91b82c3f612b5886868686604051610d589493929190611769565b60405180910390a350505050806001019050610c9d565b50610d7981610f46565b50505050505050505050565b610d8f8133610e47565b50565b610d9b826107a4565b15610dcc5781610daa5f611069565b604051635ead8eb560e01b81526004810192909252602482015260440161086d565b5f610dd660025490565b905080821015610e0357604051635433660960e01b8152600481018390526024810182905260440161086d565b610e0d8242611932565b5f93845260016020526040909320929092555050565b5f6001600160e01b03198216630271189760e51b148061069557506106958261108b565b610e518282610aca565b610e805760405163e2517d3f60e01b81526001600160a01b03821660048201526024810183905260440161086d565b5050565b610e8d8261074d565b610e9c5781610daa6002611069565b8015801590610eb15750610eaf81610772565b155b15610e805760405163121534c360e31b81526004810182905260240161086d565b5f5f856001600160a01b0316858585604051610eef929190611951565b5f6040518083038185875af1925050503d805f8114610f29576040519150601f19603f3d011682016040523d82523d5f602084013e610f2e565b606091505b5091509150610f3d82826110bf565b50505050505050565b610f4f8161074d565b610f5e5780610daa6002611069565b5f90815260016020819052604090912055565b5f610f7c8383610aca565b610ff9575f838152602081815260408083206001600160a01b03861684529091529020805460ff19166001179055610fb13390565b6001600160a01b0316826001600160a01b0316847f2f8788117e7eff1d82e926ec794901d17c78024a50270940304540a733656f0d60405160405180910390a4506001610695565b505f610695565b5f61100b8383610aca565b15610ff9575f838152602081815260408083206001600160a01b0386168085529252808320805460ff1916905551339286917ff6391f5c32d9c69d2a47ea670b442974b53935d1edc7fd64eb21e047a839171b9190a4506001610695565b5f81600381111561107c5761107c6113b1565b600160ff919091161b92915050565b5f6001600160e01b03198216637965db0b60e01b148061069557506301ffc9a760e01b6001600160e01b0319831614610695565b6060826110d4576110cf826110db565b610695565b5080610695565b8051156110eb5780518082602001fd5b604051630a12f52160e11b815260040160405180910390fd5b80356001600160a01b038116811461111a575f5ffd5b919050565b5f5f83601f84011261112f575f5ffd5b5081356001600160401b03811115611145575f5ffd5b60208301915083602082850101111561115c575f5ffd5b9250929050565b5f5f5f5f5f5f5f60c0888a031215611179575f5ffd5b61118288611104565b96506020880135955060408801356001600160401b038111156111a3575f5ffd5b6111af8a828b0161111f565b989b979a50986060810135976080820135975060a09091013595509350505050565b5f602082840312156111e1575f5ffd5b81356001600160e01b03198116811461083e575f5ffd5b5f5f5f5f5f5f60a0878903121561120d575f5ffd5b61121687611104565b95506020870135945060408701356001600160401b03811115611237575f5ffd5b61124389828a0161111f565b979a9699509760608101359660809091013595509350505050565b5f6020828403121561126e575f5ffd5b5035919050565b634e487b7160e01b5f52604160045260245ffd5b604051601f8201601f191681016001600160401b03811182821017156112b1576112b1611275565b604052919050565b5f82601f8301126112c8575f5ffd5b81356001600160401b038111156112e1576112e1611275565b6112f4601f8201601f1916602001611289565b818152846020838601011115611308575f5ffd5b816020850160208301375f918101602001919091529392505050565b5f5f5f5f60808587031215611337575f5ffd5b61134085611104565b935061134e60208601611104565b92506040850135915060608501356001600160401b0381111561136f575f5ffd5b61137b878288016112b9565b91505092959194509250565b5f5f60408385031215611398575f5ffd5b823591506113a860208401611104565b90509250929050565b634e487b7160e01b5f52602160045260245ffd5b60208101600483106113e557634e487b7160e01b5f52602160045260245ffd5b91905290565b5f5f83601f8401126113fb575f5ffd5b5081356001600160401b03811115611411575f5ffd5b6020830191508360208260051b850101111561115c575f5ffd5b5f5f5f5f5f5f5f5f5f60c08a8c031215611443575f5ffd5b89356001600160401b03811115611458575f5ffd5b6114648c828d016113eb565b909a5098505060208a01356001600160401b03811115611482575f5ffd5b61148e8c828d016113eb565b90985096505060408a01356001600160401b038111156114ac575f5ffd5b6114b88c828d016113eb565b9a9d999c50979a969997986060880135976080810135975060a0013595509350505050565b5f5f5f5f5f5f5f5f60a0898b0312156114f4575f5ffd5b88356001600160401b03811115611509575f5ffd5b6115158b828c016113eb565b90995097505060208901356001600160401b03811115611533575f5ffd5b61153f8b828c016113eb565b90975095505060408901356001600160401b0381111561155d575f5ffd5b6115698b828c016113eb565b999c989b509699959896976060870135966080013595509350505050565b5f82601f830112611596575f5ffd5b81356001600160401b038111156115af576115af611275565b8060051b6115bf60208201611289565b918252602081850181019290810190868411156115da575f5ffd5b6020860192505b838310156115fc5782358252602092830192909101906115e1565b9695505050505050565b5f5f5f5f5f60a0868803121561161a575f5ffd5b61162386611104565b945061163160208701611104565b935060408601356001600160401b0381111561164b575f5ffd5b61165788828901611587565b93505060608601356001600160401b03811115611672575f5ffd5b61167e88828901611587565b92505060808601356001600160401b03811115611699575f5ffd5b6116a5888289016112b9565b9150509295509295909350565b5f5f5f5f5f60a086880312156116c6575f5ffd5b6116cf86611104565b94506116dd60208701611104565b9350604086013592506060860135915060808601356001600160401b03811115611699575f5ffd5b81835281816020850137505f828201602090810191909152601f909101601f19169091010190565b60018060a01b038716815285602082015260a060408201525f61175460a083018688611705565b60608301949094525060800152949350505050565b60018060a01b0385168152836020820152606060408201525f6115fc606083018486611705565b634e487b7160e01b5f52603260045260245ffd5b5f602082840312156117b4575f5ffd5b61083e82611104565b5f5f8335601e198436030181126117d2575f5ffd5b8301803591506001600160401b038211156117eb575f5ffd5b60200191503681900382131561115c575f5ffd5b5f8383855260208501945060208460051b820101835f5b8681101561188757838303601f19018852813536879003601e1901811261183b575f5ffd5b86016020810190356001600160401b03811115611856575f5ffd5b803603821315611864575f5ffd5b61186f858284611705565b60209a8b019a90955093909301925050600101611816565b50909695505050505050565b60a080825281018890525f8960c08301825b8b8110156118d3576001600160a01b036118be84611104565b168252602092830192909101906001016118a5565b5083810360208501528881526001600160fb1b038911156118f2575f5ffd5b8860051b9150818a6020830137018281036020908101604085015261191a90820187896117ff565b60608401959095525050608001529695505050505050565b8082018082111561069557634e487b7160e01b5f52601160045260245ffd5b818382375f910190815291905056fea164736f6c634300081c000a
    /// ```
    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub static BYTECODE: alloy_sol_types::private::Bytes = alloy_sol_types::private::Bytes::from_static(
        b"`\x80`@R4\x80\x15a\0\x0FW__\xFD[P`@Qa\x1D\x0B8\x03\x80a\x1D\x0B\x839\x81\x01`@\x81\x90Ra\0.\x91a\x02\xFEV[\x83\x83\x83\x83a\0<_0a\x01\x83V[P`\x01`\x01`\xA0\x1B\x03\x81\x16\x15a\0XWa\0V_\x82a\x01\x83V[P[_[\x83Q\x81\x10\x15a\0\xECWa\0\xAC\x7F\xB0\x9A\xA5\xAE\xB3p,\xFDP\xB6\xB6+\xC4S&\x04\x93\x8F!$\x8A'\xA1\xD5\xCAs`\x82\xB6\x81\x9C\xC1\x85\x83\x81Q\x81\x10a\0\x99Wa\0\x99a\x03}V[` \x02` \x01\x01Qa\x01\x83` \x1B` \x1CV[Pa\0\xE3\x7F\xFDd<rq\x0Cc\xC0\x18\x02Y\xAB\xA6\xB2\xD0TQ\xE3Y\x1A$\xE5\x8Bb#\x93x\x08W&\xF7\x83\x85\x83\x81Q\x81\x10a\0\x99Wa\0\x99a\x03}V[P`\x01\x01a\0ZV[P_[\x82Q\x81\x10\x15a\x017Wa\x01.\x7F\xD8\xAA\x0F1\x94\x97\x1A*\x11fy\xF7\xC2\t\x0Fi9\xC8\xD4\xE0\x1A*\x8D~A\xD5^SQF\x9Ec\x84\x83\x81Q\x81\x10a\0\x99Wa\0\x99a\x03}V[P`\x01\x01a\0\xEFV[P`\x02\x84\x90U`@\x80Q_\x81R` \x81\x01\x86\x90R\x7F\x11\xC2ON\xAD\x16P|i\xACF\x7F\xBD^N\xED_\xB5\xC6\x99bm,\xC6\xD6d!\xDF%8\x86\xD5\x91\x01`@Q\x80\x91\x03\x90\xA1PPPPPPPPa\x03\x91V[_\x82\x81R` \x81\x81R`@\x80\x83 `\x01`\x01`\xA0\x1B\x03\x85\x16\x84R\x90\x91R\x81 T`\xFF\x16a\x02#W_\x83\x81R` \x81\x81R`@\x80\x83 `\x01`\x01`\xA0\x1B\x03\x86\x16\x84R\x90\x91R\x90 \x80T`\xFF\x19\x16`\x01\x17\x90Ua\x01\xDB3\x90V[`\x01`\x01`\xA0\x1B\x03\x16\x82`\x01`\x01`\xA0\x1B\x03\x16\x84\x7F/\x87\x88\x11~~\xFF\x1D\x82\xE9&\xECyI\x01\xD1|x\x02JP'\t@0E@\xA73eo\r`@Q`@Q\x80\x91\x03\x90\xA4P`\x01a\x02&V[P_[\x92\x91PPV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[\x80Q`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x02VW__\xFD[\x91\x90PV[_\x82`\x1F\x83\x01\x12a\x02jW__\xFD[\x81Q`\x01`\x01`@\x1B\x03\x81\x11\x15a\x02\x83Wa\x02\x83a\x02,V[`@Q`\x05\x82\x90\x1B\x90`?\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x02\xB1Wa\x02\xB1a\x02,V[`@R\x91\x82R` \x81\x85\x01\x81\x01\x92\x90\x81\x01\x86\x84\x11\x15a\x02\xCEW__\xFD[` \x86\x01\x92P[\x83\x83\x10\x15a\x02\xF4Wa\x02\xE6\x83a\x02@V[\x81R` \x92\x83\x01\x92\x01a\x02\xD5V[P\x95\x94PPPPPV[____`\x80\x85\x87\x03\x12\x15a\x03\x11W__\xFD[\x84Q` \x86\x01Q\x90\x94P`\x01`\x01`@\x1B\x03\x81\x11\x15a\x03.W__\xFD[a\x03:\x87\x82\x88\x01a\x02[V[`@\x87\x01Q\x90\x94P\x90P`\x01`\x01`@\x1B\x03\x81\x11\x15a\x03WW__\xFD[a\x03c\x87\x82\x88\x01a\x02[V[\x92PPa\x03r``\x86\x01a\x02@V[\x90P\x92\x95\x91\x94P\x92PV[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[a\x19m\x80a\x03\x9E_9_\xF3\xFE`\x80`@R`\x046\x10a\x01\xB2W_5`\xE0\x1C\x80c\x80ee\x7F\x11a\0\xE7W\x80c\xBC\x19|\x81\x11a\0\x87W\x80c\xD5Gt\x1F\x11a\0bW\x80c\xD5Gt\x1F\x14a\x05FW\x80c\xE3\x835\xE5\x14a\x05eW\x80c\xF2:na\x14a\x05xW\x80c\xF2z\x0C\x92\x14a\x05\xA3W__\xFD[\x80c\xBC\x19|\x81\x14a\x04\xD1W\x80c\xC4\xD2R\xF5\x14a\x04\xFCW\x80c\xD4\\D5\x14a\x05\x1BW__\xFD[\x80c\x91\xD1HT\x11a\0\xC2W\x80c\x91\xD1HT\x14a\x04MW\x80c\xA2\x17\xFD\xDF\x14a\x04lW\x80c\xB0\x8EQ\xC0\x14a\x04\x7FW\x80c\xB1\xC5\xF4'\x14a\x04\xB2W__\xFD[\x80c\x80ee\x7F\x14a\x03\xDCW\x80c\x8F*\x0B\xB0\x14a\x03\xFBW\x80c\x8Fa\xF4\xF5\x14a\x04\x1AW__\xFD[\x80c*\xB0\xF5)\x11a\x01RW\x80c6V\x8A\xBE\x11a\x01-W\x80c6V\x8A\xBE\x14a\x03SW\x80cXK\x15>\x14a\x03rW\x80cd\xD6#S\x14a\x03\x91W\x80cyX\0L\x14a\x03\xB0W__\xFD[\x80c*\xB0\xF5)\x14a\x02\xF6W\x80c//\xF1]\x14a\x03\x15W\x80c1\xD5\x07P\x14a\x034W__\xFD[\x80c\x13@\x08\xD3\x11a\x01\x8DW\x80c\x13@\x08\xD3\x14a\x02SW\x80c\x13\xBC\x9F \x14a\x02fW\x80c\x15\x0Bz\x02\x14a\x02\x85W\x80c$\x8A\x9C\xA3\x14a\x02\xC8W__\xFD[\x80c\x01\xD5\x06*\x14a\x01\xBDW\x80c\x01\xFF\xC9\xA7\x14a\x01\xDEW\x80c\x07\xBD\x02e\x14a\x02\x12W__\xFD[6a\x01\xB9W\0[__\xFD[4\x80\x15a\x01\xC8W__\xFD[Pa\x01\xDCa\x01\xD76`\x04a\x11cV[a\x05\xB7V[\0[4\x80\x15a\x01\xE9W__\xFD[Pa\x01\xFDa\x01\xF86`\x04a\x11\xD1V[a\x06\x8BV[`@Q\x90\x15\x15\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[4\x80\x15a\x02\x1DW__\xFD[Pa\x02E\x7F\xD8\xAA\x0F1\x94\x97\x1A*\x11fy\xF7\xC2\t\x0Fi9\xC8\xD4\xE0\x1A*\x8D~A\xD5^SQF\x9Ec\x81V[`@Q\x90\x81R` \x01a\x02\tV[a\x01\xDCa\x02a6`\x04a\x11\xF8V[a\x06\x9BV[4\x80\x15a\x02qW__\xFD[Pa\x01\xFDa\x02\x806`\x04a\x12^V[a\x07MV[4\x80\x15a\x02\x90W__\xFD[Pa\x02\xAFa\x02\x9F6`\x04a\x13$V[c\n\x85\xBD\x01`\xE1\x1B\x94\x93PPPPV[`@Q`\x01`\x01`\xE0\x1B\x03\x19\x90\x91\x16\x81R` \x01a\x02\tV[4\x80\x15a\x02\xD3W__\xFD[Pa\x02Ea\x02\xE26`\x04a\x12^V[_\x90\x81R` \x81\x90R`@\x90 `\x01\x01T\x90V[4\x80\x15a\x03\x01W__\xFD[Pa\x01\xFDa\x03\x106`\x04a\x12^V[a\x07rV[4\x80\x15a\x03 W__\xFD[Pa\x01\xDCa\x03/6`\x04a\x13\x87V[a\x07zV[4\x80\x15a\x03?W__\xFD[Pa\x01\xFDa\x03N6`\x04a\x12^V[a\x07\xA4V[4\x80\x15a\x03^W__\xFD[Pa\x01\xDCa\x03m6`\x04a\x13\x87V[a\x07\xC8V[4\x80\x15a\x03}W__\xFD[Pa\x01\xFDa\x03\x8C6`\x04a\x12^V[a\x08\0V[4\x80\x15a\x03\x9CW__\xFD[Pa\x01\xDCa\x03\xAB6`\x04a\x12^V[a\x08EV[4\x80\x15a\x03\xBBW__\xFD[Pa\x03\xCFa\x03\xCA6`\x04a\x12^V[a\x08\xB8V[`@Qa\x02\t\x91\x90a\x13\xC5V[4\x80\x15a\x03\xE7W__\xFD[Pa\x02Ea\x03\xF66`\x04a\x11\xF8V[a\t\0V[4\x80\x15a\x04\x06W__\xFD[Pa\x01\xDCa\x04\x156`\x04a\x14+V[a\t>V[4\x80\x15a\x04%W__\xFD[Pa\x02E\x7F\xB0\x9A\xA5\xAE\xB3p,\xFDP\xB6\xB6+\xC4S&\x04\x93\x8F!$\x8A'\xA1\xD5\xCAs`\x82\xB6\x81\x9C\xC1\x81V[4\x80\x15a\x04XW__\xFD[Pa\x01\xFDa\x04g6`\x04a\x13\x87V[a\n\xCAV[4\x80\x15a\x04wW__\xFD[Pa\x02E_\x81V[4\x80\x15a\x04\x8AW__\xFD[Pa\x02E\x7F\xFDd<rq\x0Cc\xC0\x18\x02Y\xAB\xA6\xB2\xD0TQ\xE3Y\x1A$\xE5\x8Bb#\x93x\x08W&\xF7\x83\x81V[4\x80\x15a\x04\xBDW__\xFD[Pa\x02Ea\x04\xCC6`\x04a\x14\xDDV[a\n\xF2V[4\x80\x15a\x04\xDCW__\xFD[Pa\x02\xAFa\x04\xEB6`\x04a\x16\x06V[c\xBC\x19|\x81`\xE0\x1B\x95\x94PPPPPV[4\x80\x15a\x05\x07W__\xFD[Pa\x01\xDCa\x05\x166`\x04a\x12^V[a\x0B6V[4\x80\x15a\x05&W__\xFD[Pa\x02Ea\x0556`\x04a\x12^V[_\x90\x81R`\x01` R`@\x90 T\x90V[4\x80\x15a\x05QW__\xFD[Pa\x01\xDCa\x05`6`\x04a\x13\x87V[a\x0B\xE0V[a\x01\xDCa\x05s6`\x04a\x14\xDDV[a\x0C\x04V[4\x80\x15a\x05\x83W__\xFD[Pa\x02\xAFa\x05\x926`\x04a\x16\xB2V[c\xF2:na`\xE0\x1B\x95\x94PPPPPV[4\x80\x15a\x05\xAEW__\xFD[P`\x02Ta\x02EV[\x7F\xB0\x9A\xA5\xAE\xB3p,\xFDP\xB6\xB6+\xC4S&\x04\x93\x8F!$\x8A'\xA1\xD5\xCAs`\x82\xB6\x81\x9C\xC1a\x05\xE1\x81a\r\x85V[_a\x05\xF0\x89\x89\x89\x89\x89\x89a\t\0V[\x90Pa\x05\xFC\x81\x84a\r\x92V[_\x81\x7FL\xF4A\x0C\xC5p@\xE4Hb\xEF\x0FE\xF3\xDDZ^\x02\xDB\x8E\xB8\xAD\xD6H\xD4\xB0\xE26\xF1\xD0}\xCA\x8B\x8B\x8B\x8B\x8B\x8A`@Qa\x067\x96\x95\x94\x93\x92\x91\x90a\x17-V[`@Q\x80\x91\x03\x90\xA3\x83\x15a\x06\x80W\x80\x7F \xFD\xA5\xFD'\xA1\xEA{\xF5\xB9V\x7F\x14:\xC5G\x0B\xB0Y7J'\xE8\xF6|\xB4O\x94om\x03\x87\x85`@Qa\x06w\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA2[PPPPPPPPPV[_a\x06\x95\x82a\x0E#V[\x92\x91PPV[\x7F\xD8\xAA\x0F1\x94\x97\x1A*\x11fy\xF7\xC2\t\x0Fi9\xC8\xD4\xE0\x1A*\x8D~A\xD5^SQF\x9Eca\x06\xC6\x81_a\n\xCAV[a\x06\xD4Wa\x06\xD4\x813a\x0EGV[_a\x06\xE3\x88\x88\x88\x88\x88\x88a\t\0V[\x90Pa\x06\xEF\x81\x85a\x0E\x84V[a\x06\xFB\x88\x88\x88\x88a\x0E\xD2V[_\x81\x7F\xC2a~\xFAi\xBA\xB6g\x82\xFA!\x95CqC8H\x9CN\x9E\x17\x82qV\n\x91\xB8,?a+X\x8A\x8A\x8A\x8A`@Qa\x072\x94\x93\x92\x91\x90a\x17iV[`@Q\x80\x91\x03\x90\xA3a\x07C\x81a\x0FFV[PPPPPPPPV[_`\x02[a\x07Z\x83a\x08\xB8V[`\x03\x81\x11\x15a\x07kWa\x07ka\x13\xB1V[\x14\x92\x91PPV[_`\x03a\x07QV[_\x82\x81R` \x81\x90R`@\x90 `\x01\x01Ta\x07\x94\x81a\r\x85V[a\x07\x9E\x83\x83a\x0FqV[PPPPV[_\x80a\x07\xAF\x83a\x08\xB8V[`\x03\x81\x11\x15a\x07\xC0Wa\x07\xC0a\x13\xB1V[\x14\x15\x92\x91PPV[`\x01`\x01`\xA0\x1B\x03\x81\x163\x14a\x07\xF1W`@Qc3K\xD9\x19`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x07\xFB\x82\x82a\x10\0V[PPPV[__a\x08\x0B\x83a\x08\xB8V[\x90P`\x01\x81`\x03\x81\x11\x15a\x08!Wa\x08!a\x13\xB1V[\x14\x80a\x08>WP`\x02\x81`\x03\x81\x11\x15a\x08<Wa\x08<a\x13\xB1V[\x14[\x93\x92PPPV[30\x81\x14a\x08vW`@Qc\xE2\x85\x0CY`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x82\x16`\x04\x82\x01R`$\x01[`@Q\x80\x91\x03\x90\xFD[`\x02T`@\x80Q\x91\x82R` \x82\x01\x84\x90R\x7F\x11\xC2ON\xAD\x16P|i\xACF\x7F\xBD^N\xED_\xB5\xC6\x99bm,\xC6\xD6d!\xDF%8\x86\xD5\x91\x01`@Q\x80\x91\x03\x90\xA1P`\x02UV[_\x81\x81R`\x01` R`@\x81 T\x80_\x03a\x08\xD5WP_\x92\x91PPV[`\x01\x81\x03a\x08\xE6WP`\x03\x92\x91PPV[B\x81\x11\x15a\x08\xF7WP`\x01\x92\x91PPV[P`\x02\x92\x91PPV[_\x86\x86\x86\x86\x86\x86`@Q` \x01a\t\x1C\x96\x95\x94\x93\x92\x91\x90a\x17-V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 \x90P\x96\x95PPPPPPV[\x7F\xB0\x9A\xA5\xAE\xB3p,\xFDP\xB6\xB6+\xC4S&\x04\x93\x8F!$\x8A'\xA1\xD5\xCAs`\x82\xB6\x81\x9C\xC1a\th\x81a\r\x85V[\x88\x87\x14\x15\x80a\twWP\x88\x85\x14\x15[\x15a\t\xA9W`@Q`\x01bO\xCD\xEF`\xE0\x1B\x03\x19\x81R`\x04\x81\x01\x8A\x90R`$\x81\x01\x86\x90R`D\x81\x01\x88\x90R`d\x01a\x08mV[_a\t\xBA\x8B\x8B\x8B\x8B\x8B\x8B\x8B\x8Ba\n\xF2V[\x90Pa\t\xC6\x81\x84a\r\x92V[_[\x8A\x81\x10\x15a\n{W\x80\x82\x7FL\xF4A\x0C\xC5p@\xE4Hb\xEF\x0FE\xF3\xDDZ^\x02\xDB\x8E\xB8\xAD\xD6H\xD4\xB0\xE26\xF1\xD0}\xCA\x8E\x8E\x85\x81\x81\x10a\n\x05Wa\n\x05a\x17\x90V[\x90P` \x02\x01` \x81\x01\x90a\n\x1A\x91\x90a\x17\xA4V[\x8D\x8D\x86\x81\x81\x10a\n,Wa\n,a\x17\x90V[\x90P` \x02\x015\x8C\x8C\x87\x81\x81\x10a\nEWa\nEa\x17\x90V[\x90P` \x02\x81\x01\x90a\nW\x91\x90a\x17\xBDV[\x8C\x8B`@Qa\nk\x96\x95\x94\x93\x92\x91\x90a\x17-V[`@Q\x80\x91\x03\x90\xA3`\x01\x01a\t\xC8V[P\x83\x15a\n\xBDW\x80\x7F \xFD\xA5\xFD'\xA1\xEA{\xF5\xB9V\x7F\x14:\xC5G\x0B\xB0Y7J'\xE8\xF6|\xB4O\x94om\x03\x87\x85`@Qa\n\xB4\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA2[PPPPPPPPPPPV[_\x91\x82R` \x82\x81R`@\x80\x84 `\x01`\x01`\xA0\x1B\x03\x93\x90\x93\x16\x84R\x91\x90R\x90 T`\xFF\x16\x90V[_\x88\x88\x88\x88\x88\x88\x88\x88`@Q` \x01a\x0B\x12\x98\x97\x96\x95\x94\x93\x92\x91\x90a\x18\x93V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 \x90P\x98\x97PPPPPPPPV[\x7F\xFDd<rq\x0Cc\xC0\x18\x02Y\xAB\xA6\xB2\xD0TQ\xE3Y\x1A$\xE5\x8Bb#\x93x\x08W&\xF7\x83a\x0B`\x81a\r\x85V[a\x0Bi\x82a\x08\0V[a\x0B\xA5W\x81a\x0Bx`\x02a\x10iV[a\x0B\x82`\x01a\x10iV[`@Qc^\xAD\x8E\xB5`\xE0\x1B\x81R`\x04\x81\x01\x93\x90\x93R\x17`$\x82\x01R`D\x01a\x08mV[_\x82\x81R`\x01` R`@\x80\x82 \x82\x90UQ\x83\x91\x7F\xBA\xA1\xEB\"\xF2\xA4\x92\xBA\x1A_\xEAa\xB8\xDFM'\xC6\xC8\xB5\xF3\x97\x1Ec\xBBX\xFA\x14\xFFr\xEE\xDBp\x91\xA2PPV[_\x82\x81R` \x81\x90R`@\x90 `\x01\x01Ta\x0B\xFA\x81a\r\x85V[a\x07\x9E\x83\x83a\x10\0V[\x7F\xD8\xAA\x0F1\x94\x97\x1A*\x11fy\xF7\xC2\t\x0Fi9\xC8\xD4\xE0\x1A*\x8D~A\xD5^SQF\x9Eca\x0C/\x81_a\n\xCAV[a\x0C=Wa\x0C=\x813a\x0EGV[\x87\x86\x14\x15\x80a\x0CLWP\x87\x84\x14\x15[\x15a\x0C~W`@Q`\x01bO\xCD\xEF`\xE0\x1B\x03\x19\x81R`\x04\x81\x01\x89\x90R`$\x81\x01\x85\x90R`D\x81\x01\x87\x90R`d\x01a\x08mV[_a\x0C\x8F\x8A\x8A\x8A\x8A\x8A\x8A\x8A\x8Aa\n\xF2V[\x90Pa\x0C\x9B\x81\x85a\x0E\x84V[_[\x89\x81\x10\x15a\roW_\x8B\x8B\x83\x81\x81\x10a\x0C\xB8Wa\x0C\xB8a\x17\x90V[\x90P` \x02\x01` \x81\x01\x90a\x0C\xCD\x91\x90a\x17\xA4V[\x90P_\x8A\x8A\x84\x81\x81\x10a\x0C\xE2Wa\x0C\xE2a\x17\x90V[\x90P` \x02\x015\x90P6_\x8A\x8A\x86\x81\x81\x10a\x0C\xFFWa\x0C\xFFa\x17\x90V[\x90P` \x02\x81\x01\x90a\r\x11\x91\x90a\x17\xBDV[\x91P\x91Pa\r!\x84\x84\x84\x84a\x0E\xD2V[\x84\x86\x7F\xC2a~\xFAi\xBA\xB6g\x82\xFA!\x95CqC8H\x9CN\x9E\x17\x82qV\n\x91\xB8,?a+X\x86\x86\x86\x86`@Qa\rX\x94\x93\x92\x91\x90a\x17iV[`@Q\x80\x91\x03\x90\xA3PPPP\x80`\x01\x01\x90Pa\x0C\x9DV[Pa\ry\x81a\x0FFV[PPPPPPPPPPV[a\r\x8F\x813a\x0EGV[PV[a\r\x9B\x82a\x07\xA4V[\x15a\r\xCCW\x81a\r\xAA_a\x10iV[`@Qc^\xAD\x8E\xB5`\xE0\x1B\x81R`\x04\x81\x01\x92\x90\x92R`$\x82\x01R`D\x01a\x08mV[_a\r\xD6`\x02T\x90V[\x90P\x80\x82\x10\x15a\x0E\x03W`@QcT3f\t`\xE0\x1B\x81R`\x04\x81\x01\x83\x90R`$\x81\x01\x82\x90R`D\x01a\x08mV[a\x0E\r\x82Ba\x192V[_\x93\x84R`\x01` R`@\x90\x93 \x92\x90\x92UPPV[_`\x01`\x01`\xE0\x1B\x03\x19\x82\x16c\x02q\x18\x97`\xE5\x1B\x14\x80a\x06\x95WPa\x06\x95\x82a\x10\x8BV[a\x0EQ\x82\x82a\n\xCAV[a\x0E\x80W`@Qc\xE2Q}?`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x82\x16`\x04\x82\x01R`$\x81\x01\x83\x90R`D\x01a\x08mV[PPV[a\x0E\x8D\x82a\x07MV[a\x0E\x9CW\x81a\r\xAA`\x02a\x10iV[\x80\x15\x80\x15\x90a\x0E\xB1WPa\x0E\xAF\x81a\x07rV[\x15[\x15a\x0E\x80W`@Qc\x12\x154\xC3`\xE3\x1B\x81R`\x04\x81\x01\x82\x90R`$\x01a\x08mV[__\x85`\x01`\x01`\xA0\x1B\x03\x16\x85\x85\x85`@Qa\x0E\xEF\x92\x91\x90a\x19QV[_`@Q\x80\x83\x03\x81\x85\x87Z\xF1\x92PPP=\x80_\x81\x14a\x0F)W`@Q\x91P`\x1F\x19`?=\x01\x16\x82\x01`@R=\x82R=_` \x84\x01>a\x0F.V[``\x91P[P\x91P\x91Pa\x0F=\x82\x82a\x10\xBFV[PPPPPPPV[a\x0FO\x81a\x07MV[a\x0F^W\x80a\r\xAA`\x02a\x10iV[_\x90\x81R`\x01` \x81\x90R`@\x90\x91 UV[_a\x0F|\x83\x83a\n\xCAV[a\x0F\xF9W_\x83\x81R` \x81\x81R`@\x80\x83 `\x01`\x01`\xA0\x1B\x03\x86\x16\x84R\x90\x91R\x90 \x80T`\xFF\x19\x16`\x01\x17\x90Ua\x0F\xB13\x90V[`\x01`\x01`\xA0\x1B\x03\x16\x82`\x01`\x01`\xA0\x1B\x03\x16\x84\x7F/\x87\x88\x11~~\xFF\x1D\x82\xE9&\xECyI\x01\xD1|x\x02JP'\t@0E@\xA73eo\r`@Q`@Q\x80\x91\x03\x90\xA4P`\x01a\x06\x95V[P_a\x06\x95V[_a\x10\x0B\x83\x83a\n\xCAV[\x15a\x0F\xF9W_\x83\x81R` \x81\x81R`@\x80\x83 `\x01`\x01`\xA0\x1B\x03\x86\x16\x80\x85R\x92R\x80\x83 \x80T`\xFF\x19\x16\x90UQ3\x92\x86\x91\x7F\xF69\x1F\\2\xD9\xC6\x9D*G\xEAg\x0BD)t\xB595\xD1\xED\xC7\xFDd\xEB!\xE0G\xA89\x17\x1B\x91\x90\xA4P`\x01a\x06\x95V[_\x81`\x03\x81\x11\x15a\x10|Wa\x10|a\x13\xB1V[`\x01`\xFF\x91\x90\x91\x16\x1B\x92\x91PPV[_`\x01`\x01`\xE0\x1B\x03\x19\x82\x16cye\xDB\x0B`\xE0\x1B\x14\x80a\x06\x95WPc\x01\xFF\xC9\xA7`\xE0\x1B`\x01`\x01`\xE0\x1B\x03\x19\x83\x16\x14a\x06\x95V[``\x82a\x10\xD4Wa\x10\xCF\x82a\x10\xDBV[a\x06\x95V[P\x80a\x06\x95V[\x80Q\x15a\x10\xEBW\x80Q\x80\x82` \x01\xFD[`@Qc\n\x12\xF5!`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x11\x1AW__\xFD[\x91\x90PV[__\x83`\x1F\x84\x01\x12a\x11/W__\xFD[P\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x11EW__\xFD[` \x83\x01\x91P\x83` \x82\x85\x01\x01\x11\x15a\x11\\W__\xFD[\x92P\x92\x90PV[_______`\xC0\x88\x8A\x03\x12\x15a\x11yW__\xFD[a\x11\x82\x88a\x11\x04V[\x96P` \x88\x015\x95P`@\x88\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x11\xA3W__\xFD[a\x11\xAF\x8A\x82\x8B\x01a\x11\x1FV[\x98\x9B\x97\x9AP\x98``\x81\x015\x97`\x80\x82\x015\x97P`\xA0\x90\x91\x015\x95P\x93PPPPV[_` \x82\x84\x03\x12\x15a\x11\xE1W__\xFD[\x815`\x01`\x01`\xE0\x1B\x03\x19\x81\x16\x81\x14a\x08>W__\xFD[______`\xA0\x87\x89\x03\x12\x15a\x12\rW__\xFD[a\x12\x16\x87a\x11\x04V[\x95P` \x87\x015\x94P`@\x87\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x127W__\xFD[a\x12C\x89\x82\x8A\x01a\x11\x1FV[\x97\x9A\x96\x99P\x97``\x81\x015\x96`\x80\x90\x91\x015\x95P\x93PPPPV[_` \x82\x84\x03\x12\x15a\x12nW__\xFD[P5\x91\x90PV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x12\xB1Wa\x12\xB1a\x12uV[`@R\x91\x90PV[_\x82`\x1F\x83\x01\x12a\x12\xC8W__\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x12\xE1Wa\x12\xE1a\x12uV[a\x12\xF4`\x1F\x82\x01`\x1F\x19\x16` \x01a\x12\x89V[\x81\x81R\x84` \x83\x86\x01\x01\x11\x15a\x13\x08W__\xFD[\x81` \x85\x01` \x83\x017_\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[____`\x80\x85\x87\x03\x12\x15a\x137W__\xFD[a\x13@\x85a\x11\x04V[\x93Pa\x13N` \x86\x01a\x11\x04V[\x92P`@\x85\x015\x91P``\x85\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x13oW__\xFD[a\x13{\x87\x82\x88\x01a\x12\xB9V[\x91PP\x92\x95\x91\x94P\x92PV[__`@\x83\x85\x03\x12\x15a\x13\x98W__\xFD[\x825\x91Pa\x13\xA8` \x84\x01a\x11\x04V[\x90P\x92P\x92\x90PV[cNH{q`\xE0\x1B_R`!`\x04R`$_\xFD[` \x81\x01`\x04\x83\x10a\x13\xE5WcNH{q`\xE0\x1B_R`!`\x04R`$_\xFD[\x91\x90R\x90V[__\x83`\x1F\x84\x01\x12a\x13\xFBW__\xFD[P\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x14\x11W__\xFD[` \x83\x01\x91P\x83` \x82`\x05\x1B\x85\x01\x01\x11\x15a\x11\\W__\xFD[_________`\xC0\x8A\x8C\x03\x12\x15a\x14CW__\xFD[\x895`\x01`\x01`@\x1B\x03\x81\x11\x15a\x14XW__\xFD[a\x14d\x8C\x82\x8D\x01a\x13\xEBV[\x90\x9AP\x98PP` \x8A\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x14\x82W__\xFD[a\x14\x8E\x8C\x82\x8D\x01a\x13\xEBV[\x90\x98P\x96PP`@\x8A\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x14\xACW__\xFD[a\x14\xB8\x8C\x82\x8D\x01a\x13\xEBV[\x9A\x9D\x99\x9CP\x97\x9A\x96\x99\x97\x98``\x88\x015\x97`\x80\x81\x015\x97P`\xA0\x015\x95P\x93PPPPV[________`\xA0\x89\x8B\x03\x12\x15a\x14\xF4W__\xFD[\x885`\x01`\x01`@\x1B\x03\x81\x11\x15a\x15\tW__\xFD[a\x15\x15\x8B\x82\x8C\x01a\x13\xEBV[\x90\x99P\x97PP` \x89\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x153W__\xFD[a\x15?\x8B\x82\x8C\x01a\x13\xEBV[\x90\x97P\x95PP`@\x89\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x15]W__\xFD[a\x15i\x8B\x82\x8C\x01a\x13\xEBV[\x99\x9C\x98\x9BP\x96\x99\x95\x98\x96\x97``\x87\x015\x96`\x80\x015\x95P\x93PPPPV[_\x82`\x1F\x83\x01\x12a\x15\x96W__\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x15\xAFWa\x15\xAFa\x12uV[\x80`\x05\x1Ba\x15\xBF` \x82\x01a\x12\x89V[\x91\x82R` \x81\x85\x01\x81\x01\x92\x90\x81\x01\x90\x86\x84\x11\x15a\x15\xDAW__\xFD[` \x86\x01\x92P[\x83\x83\x10\x15a\x15\xFCW\x825\x82R` \x92\x83\x01\x92\x90\x91\x01\x90a\x15\xE1V[\x96\x95PPPPPPV[_____`\xA0\x86\x88\x03\x12\x15a\x16\x1AW__\xFD[a\x16#\x86a\x11\x04V[\x94Pa\x161` \x87\x01a\x11\x04V[\x93P`@\x86\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x16KW__\xFD[a\x16W\x88\x82\x89\x01a\x15\x87V[\x93PP``\x86\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x16rW__\xFD[a\x16~\x88\x82\x89\x01a\x15\x87V[\x92PP`\x80\x86\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x16\x99W__\xFD[a\x16\xA5\x88\x82\x89\x01a\x12\xB9V[\x91PP\x92\x95P\x92\x95\x90\x93PV[_____`\xA0\x86\x88\x03\x12\x15a\x16\xC6W__\xFD[a\x16\xCF\x86a\x11\x04V[\x94Pa\x16\xDD` \x87\x01a\x11\x04V[\x93P`@\x86\x015\x92P``\x86\x015\x91P`\x80\x86\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x16\x99W__\xFD[\x81\x83R\x81\x81` \x85\x017P_\x82\x82\x01` \x90\x81\x01\x91\x90\x91R`\x1F\x90\x91\x01`\x1F\x19\x16\x90\x91\x01\x01\x90V[`\x01\x80`\xA0\x1B\x03\x87\x16\x81R\x85` \x82\x01R`\xA0`@\x82\x01R_a\x17T`\xA0\x83\x01\x86\x88a\x17\x05V[``\x83\x01\x94\x90\x94RP`\x80\x01R\x94\x93PPPPV[`\x01\x80`\xA0\x1B\x03\x85\x16\x81R\x83` \x82\x01R```@\x82\x01R_a\x15\xFC``\x83\x01\x84\x86a\x17\x05V[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[_` \x82\x84\x03\x12\x15a\x17\xB4W__\xFD[a\x08>\x82a\x11\x04V[__\x835`\x1E\x19\x846\x03\x01\x81\x12a\x17\xD2W__\xFD[\x83\x01\x805\x91P`\x01`\x01`@\x1B\x03\x82\x11\x15a\x17\xEBW__\xFD[` \x01\x91P6\x81\x90\x03\x82\x13\x15a\x11\\W__\xFD[_\x83\x83\x85R` \x85\x01\x94P` \x84`\x05\x1B\x82\x01\x01\x83_[\x86\x81\x10\x15a\x18\x87W\x83\x83\x03`\x1F\x19\x01\x88R\x8156\x87\x90\x03`\x1E\x19\x01\x81\x12a\x18;W__\xFD[\x86\x01` \x81\x01\x905`\x01`\x01`@\x1B\x03\x81\x11\x15a\x18VW__\xFD[\x806\x03\x82\x13\x15a\x18dW__\xFD[a\x18o\x85\x82\x84a\x17\x05V[` \x9A\x8B\x01\x9A\x90\x95P\x93\x90\x93\x01\x92PP`\x01\x01a\x18\x16V[P\x90\x96\x95PPPPPPV[`\xA0\x80\x82R\x81\x01\x88\x90R_\x89`\xC0\x83\x01\x82[\x8B\x81\x10\x15a\x18\xD3W`\x01`\x01`\xA0\x1B\x03a\x18\xBE\x84a\x11\x04V[\x16\x82R` \x92\x83\x01\x92\x90\x91\x01\x90`\x01\x01a\x18\xA5V[P\x83\x81\x03` \x85\x01R\x88\x81R`\x01`\x01`\xFB\x1B\x03\x89\x11\x15a\x18\xF2W__\xFD[\x88`\x05\x1B\x91P\x81\x8A` \x83\x017\x01\x82\x81\x03` \x90\x81\x01`@\x85\x01Ra\x19\x1A\x90\x82\x01\x87\x89a\x17\xFFV[``\x84\x01\x95\x90\x95RPP`\x80\x01R\x96\x95PPPPPPV[\x80\x82\x01\x80\x82\x11\x15a\x06\x95WcNH{q`\xE0\x1B_R`\x11`\x04R`$_\xFD[\x81\x83\x827_\x91\x01\x90\x81R\x91\x90PV\xFE\xA1dsolcC\0\x08\x1C\0\n",
    );
    /// The runtime bytecode of the contract, as deployed on the network.
    ///
    /// ```text
    ///0x6080604052600436106101b2575f3560e01c80638065657f116100e7578063bc197c8111610087578063d547741f11610062578063d547741f14610546578063e38335e514610565578063f23a6e6114610578578063f27a0c92146105a3575f5ffd5b8063bc197c81146104d1578063c4d252f5146104fc578063d45c44351461051b575f5ffd5b806391d14854116100c257806391d148541461044d578063a217fddf1461046c578063b08e51c01461047f578063b1c5f427146104b2575f5ffd5b80638065657f146103dc5780638f2a0bb0146103fb5780638f61f4f51461041a575f5ffd5b80632ab0f5291161015257806336568abe1161012d57806336568abe14610353578063584b153e1461037257806364d62353146103915780637958004c146103b0575f5ffd5b80632ab0f529146102f65780632f2ff15d1461031557806331d5075014610334575f5ffd5b8063134008d31161018d578063134008d31461025357806313bc9f2014610266578063150b7a0214610285578063248a9ca3146102c8575f5ffd5b806301d5062a146101bd57806301ffc9a7146101de57806307bd026514610212575f5ffd5b366101b957005b5f5ffd5b3480156101c8575f5ffd5b506101dc6101d7366004611163565b6105b7565b005b3480156101e9575f5ffd5b506101fd6101f83660046111d1565b61068b565b60405190151581526020015b60405180910390f35b34801561021d575f5ffd5b506102457fd8aa0f3194971a2a116679f7c2090f6939c8d4e01a2a8d7e41d55e5351469e6381565b604051908152602001610209565b6101dc6102613660046111f8565b61069b565b348015610271575f5ffd5b506101fd61028036600461125e565b61074d565b348015610290575f5ffd5b506102af61029f366004611324565b630a85bd0160e11b949350505050565b6040516001600160e01b03199091168152602001610209565b3480156102d3575f5ffd5b506102456102e236600461125e565b5f9081526020819052604090206001015490565b348015610301575f5ffd5b506101fd61031036600461125e565b610772565b348015610320575f5ffd5b506101dc61032f366004611387565b61077a565b34801561033f575f5ffd5b506101fd61034e36600461125e565b6107a4565b34801561035e575f5ffd5b506101dc61036d366004611387565b6107c8565b34801561037d575f5ffd5b506101fd61038c36600461125e565b610800565b34801561039c575f5ffd5b506101dc6103ab36600461125e565b610845565b3480156103bb575f5ffd5b506103cf6103ca36600461125e565b6108b8565b60405161020991906113c5565b3480156103e7575f5ffd5b506102456103f63660046111f8565b610900565b348015610406575f5ffd5b506101dc61041536600461142b565b61093e565b348015610425575f5ffd5b506102457fb09aa5aeb3702cfd50b6b62bc4532604938f21248a27a1d5ca736082b6819cc181565b348015610458575f5ffd5b506101fd610467366004611387565b610aca565b348015610477575f5ffd5b506102455f81565b34801561048a575f5ffd5b506102457ffd643c72710c63c0180259aba6b2d05451e3591a24e58b62239378085726f78381565b3480156104bd575f5ffd5b506102456104cc3660046114dd565b610af2565b3480156104dc575f5ffd5b506102af6104eb366004611606565b63bc197c8160e01b95945050505050565b348015610507575f5ffd5b506101dc61051636600461125e565b610b36565b348015610526575f5ffd5b5061024561053536600461125e565b5f9081526001602052604090205490565b348015610551575f5ffd5b506101dc610560366004611387565b610be0565b6101dc6105733660046114dd565b610c04565b348015610583575f5ffd5b506102af6105923660046116b2565b63f23a6e6160e01b95945050505050565b3480156105ae575f5ffd5b50600254610245565b7fb09aa5aeb3702cfd50b6b62bc4532604938f21248a27a1d5ca736082b6819cc16105e181610d85565b5f6105f0898989898989610900565b90506105fc8184610d92565b5f817f4cf4410cc57040e44862ef0f45f3dd5a5e02db8eb8add648d4b0e236f1d07dca8b8b8b8b8b8a6040516106379695949392919061172d565b60405180910390a3831561068057807f20fda5fd27a1ea7bf5b9567f143ac5470bb059374a27e8f67cb44f946f6d03878560405161067791815260200190565b60405180910390a25b505050505050505050565b5f61069582610e23565b92915050565b7fd8aa0f3194971a2a116679f7c2090f6939c8d4e01a2a8d7e41d55e5351469e636106c6815f610aca565b6106d4576106d48133610e47565b5f6106e3888888888888610900565b90506106ef8185610e84565b6106fb88888888610ed2565b5f817fc2617efa69bab66782fa219543714338489c4e9e178271560a91b82c3f612b588a8a8a8a6040516107329493929190611769565b60405180910390a361074381610f46565b5050505050505050565b5f60025b61075a836108b8565b600381111561076b5761076b6113b1565b1492915050565b5f6003610751565b5f8281526020819052604090206001015461079481610d85565b61079e8383610f71565b50505050565b5f806107af836108b8565b60038111156107c0576107c06113b1565b141592915050565b6001600160a01b03811633146107f15760405163334bd91960e11b815260040160405180910390fd5b6107fb8282611000565b505050565b5f5f61080b836108b8565b90506001816003811115610821576108216113b1565b148061083e5750600281600381111561083c5761083c6113b1565b145b9392505050565b333081146108765760405163e2850c5960e01b81526001600160a01b03821660048201526024015b60405180910390fd5b60025460408051918252602082018490527f11c24f4ead16507c69ac467fbd5e4eed5fb5c699626d2cc6d66421df253886d5910160405180910390a150600255565b5f81815260016020526040812054805f036108d557505f92915050565b600181036108e65750600392915050565b428111156108f75750600192915050565b50600292915050565b5f86868686868660405160200161091c9695949392919061172d565b6040516020818303038152906040528051906020012090509695505050505050565b7fb09aa5aeb3702cfd50b6b62bc4532604938f21248a27a1d5ca736082b6819cc161096881610d85565b88871415806109775750888514155b156109a9576040516001624fcdef60e01b03198152600481018a9052602481018690526044810188905260640161086d565b5f6109ba8b8b8b8b8b8b8b8b610af2565b90506109c68184610d92565b5f5b8a811015610a7b5780827f4cf4410cc57040e44862ef0f45f3dd5a5e02db8eb8add648d4b0e236f1d07dca8e8e85818110610a0557610a05611790565b9050602002016020810190610a1a91906117a4565b8d8d86818110610a2c57610a2c611790565b905060200201358c8c87818110610a4557610a45611790565b9050602002810190610a5791906117bd565b8c8b604051610a6b9695949392919061172d565b60405180910390a36001016109c8565b508315610abd57807f20fda5fd27a1ea7bf5b9567f143ac5470bb059374a27e8f67cb44f946f6d038785604051610ab491815260200190565b60405180910390a25b5050505050505050505050565b5f918252602082815260408084206001600160a01b0393909316845291905290205460ff1690565b5f8888888888888888604051602001610b12989796959493929190611893565b60405160208183030381529060405280519060200120905098975050505050505050565b7ffd643c72710c63c0180259aba6b2d05451e3591a24e58b62239378085726f783610b6081610d85565b610b6982610800565b610ba55781610b786002611069565b610b826001611069565b604051635ead8eb560e01b8152600481019390935217602482015260440161086d565b5f828152600160205260408082208290555183917fbaa1eb22f2a492ba1a5fea61b8df4d27c6c8b5f3971e63bb58fa14ff72eedb7091a25050565b5f82815260208190526040902060010154610bfa81610d85565b61079e8383611000565b7fd8aa0f3194971a2a116679f7c2090f6939c8d4e01a2a8d7e41d55e5351469e63610c2f815f610aca565b610c3d57610c3d8133610e47565b8786141580610c4c5750878414155b15610c7e576040516001624fcdef60e01b0319815260048101899052602481018590526044810187905260640161086d565b5f610c8f8a8a8a8a8a8a8a8a610af2565b9050610c9b8185610e84565b5f5b89811015610d6f575f8b8b83818110610cb857610cb8611790565b9050602002016020810190610ccd91906117a4565b90505f8a8a84818110610ce257610ce2611790565b905060200201359050365f8a8a86818110610cff57610cff611790565b9050602002810190610d1191906117bd565b91509150610d2184848484610ed2565b84867fc2617efa69bab66782fa219543714338489c4e9e178271560a91b82c3f612b5886868686604051610d589493929190611769565b60405180910390a350505050806001019050610c9d565b50610d7981610f46565b50505050505050505050565b610d8f8133610e47565b50565b610d9b826107a4565b15610dcc5781610daa5f611069565b604051635ead8eb560e01b81526004810192909252602482015260440161086d565b5f610dd660025490565b905080821015610e0357604051635433660960e01b8152600481018390526024810182905260440161086d565b610e0d8242611932565b5f93845260016020526040909320929092555050565b5f6001600160e01b03198216630271189760e51b148061069557506106958261108b565b610e518282610aca565b610e805760405163e2517d3f60e01b81526001600160a01b03821660048201526024810183905260440161086d565b5050565b610e8d8261074d565b610e9c5781610daa6002611069565b8015801590610eb15750610eaf81610772565b155b15610e805760405163121534c360e31b81526004810182905260240161086d565b5f5f856001600160a01b0316858585604051610eef929190611951565b5f6040518083038185875af1925050503d805f8114610f29576040519150601f19603f3d011682016040523d82523d5f602084013e610f2e565b606091505b5091509150610f3d82826110bf565b50505050505050565b610f4f8161074d565b610f5e5780610daa6002611069565b5f90815260016020819052604090912055565b5f610f7c8383610aca565b610ff9575f838152602081815260408083206001600160a01b03861684529091529020805460ff19166001179055610fb13390565b6001600160a01b0316826001600160a01b0316847f2f8788117e7eff1d82e926ec794901d17c78024a50270940304540a733656f0d60405160405180910390a4506001610695565b505f610695565b5f61100b8383610aca565b15610ff9575f838152602081815260408083206001600160a01b0386168085529252808320805460ff1916905551339286917ff6391f5c32d9c69d2a47ea670b442974b53935d1edc7fd64eb21e047a839171b9190a4506001610695565b5f81600381111561107c5761107c6113b1565b600160ff919091161b92915050565b5f6001600160e01b03198216637965db0b60e01b148061069557506301ffc9a760e01b6001600160e01b0319831614610695565b6060826110d4576110cf826110db565b610695565b5080610695565b8051156110eb5780518082602001fd5b604051630a12f52160e11b815260040160405180910390fd5b80356001600160a01b038116811461111a575f5ffd5b919050565b5f5f83601f84011261112f575f5ffd5b5081356001600160401b03811115611145575f5ffd5b60208301915083602082850101111561115c575f5ffd5b9250929050565b5f5f5f5f5f5f5f60c0888a031215611179575f5ffd5b61118288611104565b96506020880135955060408801356001600160401b038111156111a3575f5ffd5b6111af8a828b0161111f565b989b979a50986060810135976080820135975060a09091013595509350505050565b5f602082840312156111e1575f5ffd5b81356001600160e01b03198116811461083e575f5ffd5b5f5f5f5f5f5f60a0878903121561120d575f5ffd5b61121687611104565b95506020870135945060408701356001600160401b03811115611237575f5ffd5b61124389828a0161111f565b979a9699509760608101359660809091013595509350505050565b5f6020828403121561126e575f5ffd5b5035919050565b634e487b7160e01b5f52604160045260245ffd5b604051601f8201601f191681016001600160401b03811182821017156112b1576112b1611275565b604052919050565b5f82601f8301126112c8575f5ffd5b81356001600160401b038111156112e1576112e1611275565b6112f4601f8201601f1916602001611289565b818152846020838601011115611308575f5ffd5b816020850160208301375f918101602001919091529392505050565b5f5f5f5f60808587031215611337575f5ffd5b61134085611104565b935061134e60208601611104565b92506040850135915060608501356001600160401b0381111561136f575f5ffd5b61137b878288016112b9565b91505092959194509250565b5f5f60408385031215611398575f5ffd5b823591506113a860208401611104565b90509250929050565b634e487b7160e01b5f52602160045260245ffd5b60208101600483106113e557634e487b7160e01b5f52602160045260245ffd5b91905290565b5f5f83601f8401126113fb575f5ffd5b5081356001600160401b03811115611411575f5ffd5b6020830191508360208260051b850101111561115c575f5ffd5b5f5f5f5f5f5f5f5f5f60c08a8c031215611443575f5ffd5b89356001600160401b03811115611458575f5ffd5b6114648c828d016113eb565b909a5098505060208a01356001600160401b03811115611482575f5ffd5b61148e8c828d016113eb565b90985096505060408a01356001600160401b038111156114ac575f5ffd5b6114b88c828d016113eb565b9a9d999c50979a969997986060880135976080810135975060a0013595509350505050565b5f5f5f5f5f5f5f5f60a0898b0312156114f4575f5ffd5b88356001600160401b03811115611509575f5ffd5b6115158b828c016113eb565b90995097505060208901356001600160401b03811115611533575f5ffd5b61153f8b828c016113eb565b90975095505060408901356001600160401b0381111561155d575f5ffd5b6115698b828c016113eb565b999c989b509699959896976060870135966080013595509350505050565b5f82601f830112611596575f5ffd5b81356001600160401b038111156115af576115af611275565b8060051b6115bf60208201611289565b918252602081850181019290810190868411156115da575f5ffd5b6020860192505b838310156115fc5782358252602092830192909101906115e1565b9695505050505050565b5f5f5f5f5f60a0868803121561161a575f5ffd5b61162386611104565b945061163160208701611104565b935060408601356001600160401b0381111561164b575f5ffd5b61165788828901611587565b93505060608601356001600160401b03811115611672575f5ffd5b61167e88828901611587565b92505060808601356001600160401b03811115611699575f5ffd5b6116a5888289016112b9565b9150509295509295909350565b5f5f5f5f5f60a086880312156116c6575f5ffd5b6116cf86611104565b94506116dd60208701611104565b9350604086013592506060860135915060808601356001600160401b03811115611699575f5ffd5b81835281816020850137505f828201602090810191909152601f909101601f19169091010190565b60018060a01b038716815285602082015260a060408201525f61175460a083018688611705565b60608301949094525060800152949350505050565b60018060a01b0385168152836020820152606060408201525f6115fc606083018486611705565b634e487b7160e01b5f52603260045260245ffd5b5f602082840312156117b4575f5ffd5b61083e82611104565b5f5f8335601e198436030181126117d2575f5ffd5b8301803591506001600160401b038211156117eb575f5ffd5b60200191503681900382131561115c575f5ffd5b5f8383855260208501945060208460051b820101835f5b8681101561188757838303601f19018852813536879003601e1901811261183b575f5ffd5b86016020810190356001600160401b03811115611856575f5ffd5b803603821315611864575f5ffd5b61186f858284611705565b60209a8b019a90955093909301925050600101611816565b50909695505050505050565b60a080825281018890525f8960c08301825b8b8110156118d3576001600160a01b036118be84611104565b168252602092830192909101906001016118a5565b5083810360208501528881526001600160fb1b038911156118f2575f5ffd5b8860051b9150818a6020830137018281036020908101604085015261191a90820187896117ff565b60608401959095525050608001529695505050505050565b8082018082111561069557634e487b7160e01b5f52601160045260245ffd5b818382375f910190815291905056fea164736f6c634300081c000a
    /// ```
    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub static DEPLOYED_BYTECODE: alloy_sol_types::private::Bytes = alloy_sol_types::private::Bytes::from_static(
        b"`\x80`@R`\x046\x10a\x01\xB2W_5`\xE0\x1C\x80c\x80ee\x7F\x11a\0\xE7W\x80c\xBC\x19|\x81\x11a\0\x87W\x80c\xD5Gt\x1F\x11a\0bW\x80c\xD5Gt\x1F\x14a\x05FW\x80c\xE3\x835\xE5\x14a\x05eW\x80c\xF2:na\x14a\x05xW\x80c\xF2z\x0C\x92\x14a\x05\xA3W__\xFD[\x80c\xBC\x19|\x81\x14a\x04\xD1W\x80c\xC4\xD2R\xF5\x14a\x04\xFCW\x80c\xD4\\D5\x14a\x05\x1BW__\xFD[\x80c\x91\xD1HT\x11a\0\xC2W\x80c\x91\xD1HT\x14a\x04MW\x80c\xA2\x17\xFD\xDF\x14a\x04lW\x80c\xB0\x8EQ\xC0\x14a\x04\x7FW\x80c\xB1\xC5\xF4'\x14a\x04\xB2W__\xFD[\x80c\x80ee\x7F\x14a\x03\xDCW\x80c\x8F*\x0B\xB0\x14a\x03\xFBW\x80c\x8Fa\xF4\xF5\x14a\x04\x1AW__\xFD[\x80c*\xB0\xF5)\x11a\x01RW\x80c6V\x8A\xBE\x11a\x01-W\x80c6V\x8A\xBE\x14a\x03SW\x80cXK\x15>\x14a\x03rW\x80cd\xD6#S\x14a\x03\x91W\x80cyX\0L\x14a\x03\xB0W__\xFD[\x80c*\xB0\xF5)\x14a\x02\xF6W\x80c//\xF1]\x14a\x03\x15W\x80c1\xD5\x07P\x14a\x034W__\xFD[\x80c\x13@\x08\xD3\x11a\x01\x8DW\x80c\x13@\x08\xD3\x14a\x02SW\x80c\x13\xBC\x9F \x14a\x02fW\x80c\x15\x0Bz\x02\x14a\x02\x85W\x80c$\x8A\x9C\xA3\x14a\x02\xC8W__\xFD[\x80c\x01\xD5\x06*\x14a\x01\xBDW\x80c\x01\xFF\xC9\xA7\x14a\x01\xDEW\x80c\x07\xBD\x02e\x14a\x02\x12W__\xFD[6a\x01\xB9W\0[__\xFD[4\x80\x15a\x01\xC8W__\xFD[Pa\x01\xDCa\x01\xD76`\x04a\x11cV[a\x05\xB7V[\0[4\x80\x15a\x01\xE9W__\xFD[Pa\x01\xFDa\x01\xF86`\x04a\x11\xD1V[a\x06\x8BV[`@Q\x90\x15\x15\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[4\x80\x15a\x02\x1DW__\xFD[Pa\x02E\x7F\xD8\xAA\x0F1\x94\x97\x1A*\x11fy\xF7\xC2\t\x0Fi9\xC8\xD4\xE0\x1A*\x8D~A\xD5^SQF\x9Ec\x81V[`@Q\x90\x81R` \x01a\x02\tV[a\x01\xDCa\x02a6`\x04a\x11\xF8V[a\x06\x9BV[4\x80\x15a\x02qW__\xFD[Pa\x01\xFDa\x02\x806`\x04a\x12^V[a\x07MV[4\x80\x15a\x02\x90W__\xFD[Pa\x02\xAFa\x02\x9F6`\x04a\x13$V[c\n\x85\xBD\x01`\xE1\x1B\x94\x93PPPPV[`@Q`\x01`\x01`\xE0\x1B\x03\x19\x90\x91\x16\x81R` \x01a\x02\tV[4\x80\x15a\x02\xD3W__\xFD[Pa\x02Ea\x02\xE26`\x04a\x12^V[_\x90\x81R` \x81\x90R`@\x90 `\x01\x01T\x90V[4\x80\x15a\x03\x01W__\xFD[Pa\x01\xFDa\x03\x106`\x04a\x12^V[a\x07rV[4\x80\x15a\x03 W__\xFD[Pa\x01\xDCa\x03/6`\x04a\x13\x87V[a\x07zV[4\x80\x15a\x03?W__\xFD[Pa\x01\xFDa\x03N6`\x04a\x12^V[a\x07\xA4V[4\x80\x15a\x03^W__\xFD[Pa\x01\xDCa\x03m6`\x04a\x13\x87V[a\x07\xC8V[4\x80\x15a\x03}W__\xFD[Pa\x01\xFDa\x03\x8C6`\x04a\x12^V[a\x08\0V[4\x80\x15a\x03\x9CW__\xFD[Pa\x01\xDCa\x03\xAB6`\x04a\x12^V[a\x08EV[4\x80\x15a\x03\xBBW__\xFD[Pa\x03\xCFa\x03\xCA6`\x04a\x12^V[a\x08\xB8V[`@Qa\x02\t\x91\x90a\x13\xC5V[4\x80\x15a\x03\xE7W__\xFD[Pa\x02Ea\x03\xF66`\x04a\x11\xF8V[a\t\0V[4\x80\x15a\x04\x06W__\xFD[Pa\x01\xDCa\x04\x156`\x04a\x14+V[a\t>V[4\x80\x15a\x04%W__\xFD[Pa\x02E\x7F\xB0\x9A\xA5\xAE\xB3p,\xFDP\xB6\xB6+\xC4S&\x04\x93\x8F!$\x8A'\xA1\xD5\xCAs`\x82\xB6\x81\x9C\xC1\x81V[4\x80\x15a\x04XW__\xFD[Pa\x01\xFDa\x04g6`\x04a\x13\x87V[a\n\xCAV[4\x80\x15a\x04wW__\xFD[Pa\x02E_\x81V[4\x80\x15a\x04\x8AW__\xFD[Pa\x02E\x7F\xFDd<rq\x0Cc\xC0\x18\x02Y\xAB\xA6\xB2\xD0TQ\xE3Y\x1A$\xE5\x8Bb#\x93x\x08W&\xF7\x83\x81V[4\x80\x15a\x04\xBDW__\xFD[Pa\x02Ea\x04\xCC6`\x04a\x14\xDDV[a\n\xF2V[4\x80\x15a\x04\xDCW__\xFD[Pa\x02\xAFa\x04\xEB6`\x04a\x16\x06V[c\xBC\x19|\x81`\xE0\x1B\x95\x94PPPPPV[4\x80\x15a\x05\x07W__\xFD[Pa\x01\xDCa\x05\x166`\x04a\x12^V[a\x0B6V[4\x80\x15a\x05&W__\xFD[Pa\x02Ea\x0556`\x04a\x12^V[_\x90\x81R`\x01` R`@\x90 T\x90V[4\x80\x15a\x05QW__\xFD[Pa\x01\xDCa\x05`6`\x04a\x13\x87V[a\x0B\xE0V[a\x01\xDCa\x05s6`\x04a\x14\xDDV[a\x0C\x04V[4\x80\x15a\x05\x83W__\xFD[Pa\x02\xAFa\x05\x926`\x04a\x16\xB2V[c\xF2:na`\xE0\x1B\x95\x94PPPPPV[4\x80\x15a\x05\xAEW__\xFD[P`\x02Ta\x02EV[\x7F\xB0\x9A\xA5\xAE\xB3p,\xFDP\xB6\xB6+\xC4S&\x04\x93\x8F!$\x8A'\xA1\xD5\xCAs`\x82\xB6\x81\x9C\xC1a\x05\xE1\x81a\r\x85V[_a\x05\xF0\x89\x89\x89\x89\x89\x89a\t\0V[\x90Pa\x05\xFC\x81\x84a\r\x92V[_\x81\x7FL\xF4A\x0C\xC5p@\xE4Hb\xEF\x0FE\xF3\xDDZ^\x02\xDB\x8E\xB8\xAD\xD6H\xD4\xB0\xE26\xF1\xD0}\xCA\x8B\x8B\x8B\x8B\x8B\x8A`@Qa\x067\x96\x95\x94\x93\x92\x91\x90a\x17-V[`@Q\x80\x91\x03\x90\xA3\x83\x15a\x06\x80W\x80\x7F \xFD\xA5\xFD'\xA1\xEA{\xF5\xB9V\x7F\x14:\xC5G\x0B\xB0Y7J'\xE8\xF6|\xB4O\x94om\x03\x87\x85`@Qa\x06w\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA2[PPPPPPPPPV[_a\x06\x95\x82a\x0E#V[\x92\x91PPV[\x7F\xD8\xAA\x0F1\x94\x97\x1A*\x11fy\xF7\xC2\t\x0Fi9\xC8\xD4\xE0\x1A*\x8D~A\xD5^SQF\x9Eca\x06\xC6\x81_a\n\xCAV[a\x06\xD4Wa\x06\xD4\x813a\x0EGV[_a\x06\xE3\x88\x88\x88\x88\x88\x88a\t\0V[\x90Pa\x06\xEF\x81\x85a\x0E\x84V[a\x06\xFB\x88\x88\x88\x88a\x0E\xD2V[_\x81\x7F\xC2a~\xFAi\xBA\xB6g\x82\xFA!\x95CqC8H\x9CN\x9E\x17\x82qV\n\x91\xB8,?a+X\x8A\x8A\x8A\x8A`@Qa\x072\x94\x93\x92\x91\x90a\x17iV[`@Q\x80\x91\x03\x90\xA3a\x07C\x81a\x0FFV[PPPPPPPPV[_`\x02[a\x07Z\x83a\x08\xB8V[`\x03\x81\x11\x15a\x07kWa\x07ka\x13\xB1V[\x14\x92\x91PPV[_`\x03a\x07QV[_\x82\x81R` \x81\x90R`@\x90 `\x01\x01Ta\x07\x94\x81a\r\x85V[a\x07\x9E\x83\x83a\x0FqV[PPPPV[_\x80a\x07\xAF\x83a\x08\xB8V[`\x03\x81\x11\x15a\x07\xC0Wa\x07\xC0a\x13\xB1V[\x14\x15\x92\x91PPV[`\x01`\x01`\xA0\x1B\x03\x81\x163\x14a\x07\xF1W`@Qc3K\xD9\x19`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x07\xFB\x82\x82a\x10\0V[PPPV[__a\x08\x0B\x83a\x08\xB8V[\x90P`\x01\x81`\x03\x81\x11\x15a\x08!Wa\x08!a\x13\xB1V[\x14\x80a\x08>WP`\x02\x81`\x03\x81\x11\x15a\x08<Wa\x08<a\x13\xB1V[\x14[\x93\x92PPPV[30\x81\x14a\x08vW`@Qc\xE2\x85\x0CY`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x82\x16`\x04\x82\x01R`$\x01[`@Q\x80\x91\x03\x90\xFD[`\x02T`@\x80Q\x91\x82R` \x82\x01\x84\x90R\x7F\x11\xC2ON\xAD\x16P|i\xACF\x7F\xBD^N\xED_\xB5\xC6\x99bm,\xC6\xD6d!\xDF%8\x86\xD5\x91\x01`@Q\x80\x91\x03\x90\xA1P`\x02UV[_\x81\x81R`\x01` R`@\x81 T\x80_\x03a\x08\xD5WP_\x92\x91PPV[`\x01\x81\x03a\x08\xE6WP`\x03\x92\x91PPV[B\x81\x11\x15a\x08\xF7WP`\x01\x92\x91PPV[P`\x02\x92\x91PPV[_\x86\x86\x86\x86\x86\x86`@Q` \x01a\t\x1C\x96\x95\x94\x93\x92\x91\x90a\x17-V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 \x90P\x96\x95PPPPPPV[\x7F\xB0\x9A\xA5\xAE\xB3p,\xFDP\xB6\xB6+\xC4S&\x04\x93\x8F!$\x8A'\xA1\xD5\xCAs`\x82\xB6\x81\x9C\xC1a\th\x81a\r\x85V[\x88\x87\x14\x15\x80a\twWP\x88\x85\x14\x15[\x15a\t\xA9W`@Q`\x01bO\xCD\xEF`\xE0\x1B\x03\x19\x81R`\x04\x81\x01\x8A\x90R`$\x81\x01\x86\x90R`D\x81\x01\x88\x90R`d\x01a\x08mV[_a\t\xBA\x8B\x8B\x8B\x8B\x8B\x8B\x8B\x8Ba\n\xF2V[\x90Pa\t\xC6\x81\x84a\r\x92V[_[\x8A\x81\x10\x15a\n{W\x80\x82\x7FL\xF4A\x0C\xC5p@\xE4Hb\xEF\x0FE\xF3\xDDZ^\x02\xDB\x8E\xB8\xAD\xD6H\xD4\xB0\xE26\xF1\xD0}\xCA\x8E\x8E\x85\x81\x81\x10a\n\x05Wa\n\x05a\x17\x90V[\x90P` \x02\x01` \x81\x01\x90a\n\x1A\x91\x90a\x17\xA4V[\x8D\x8D\x86\x81\x81\x10a\n,Wa\n,a\x17\x90V[\x90P` \x02\x015\x8C\x8C\x87\x81\x81\x10a\nEWa\nEa\x17\x90V[\x90P` \x02\x81\x01\x90a\nW\x91\x90a\x17\xBDV[\x8C\x8B`@Qa\nk\x96\x95\x94\x93\x92\x91\x90a\x17-V[`@Q\x80\x91\x03\x90\xA3`\x01\x01a\t\xC8V[P\x83\x15a\n\xBDW\x80\x7F \xFD\xA5\xFD'\xA1\xEA{\xF5\xB9V\x7F\x14:\xC5G\x0B\xB0Y7J'\xE8\xF6|\xB4O\x94om\x03\x87\x85`@Qa\n\xB4\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA2[PPPPPPPPPPPV[_\x91\x82R` \x82\x81R`@\x80\x84 `\x01`\x01`\xA0\x1B\x03\x93\x90\x93\x16\x84R\x91\x90R\x90 T`\xFF\x16\x90V[_\x88\x88\x88\x88\x88\x88\x88\x88`@Q` \x01a\x0B\x12\x98\x97\x96\x95\x94\x93\x92\x91\x90a\x18\x93V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 \x90P\x98\x97PPPPPPPPV[\x7F\xFDd<rq\x0Cc\xC0\x18\x02Y\xAB\xA6\xB2\xD0TQ\xE3Y\x1A$\xE5\x8Bb#\x93x\x08W&\xF7\x83a\x0B`\x81a\r\x85V[a\x0Bi\x82a\x08\0V[a\x0B\xA5W\x81a\x0Bx`\x02a\x10iV[a\x0B\x82`\x01a\x10iV[`@Qc^\xAD\x8E\xB5`\xE0\x1B\x81R`\x04\x81\x01\x93\x90\x93R\x17`$\x82\x01R`D\x01a\x08mV[_\x82\x81R`\x01` R`@\x80\x82 \x82\x90UQ\x83\x91\x7F\xBA\xA1\xEB\"\xF2\xA4\x92\xBA\x1A_\xEAa\xB8\xDFM'\xC6\xC8\xB5\xF3\x97\x1Ec\xBBX\xFA\x14\xFFr\xEE\xDBp\x91\xA2PPV[_\x82\x81R` \x81\x90R`@\x90 `\x01\x01Ta\x0B\xFA\x81a\r\x85V[a\x07\x9E\x83\x83a\x10\0V[\x7F\xD8\xAA\x0F1\x94\x97\x1A*\x11fy\xF7\xC2\t\x0Fi9\xC8\xD4\xE0\x1A*\x8D~A\xD5^SQF\x9Eca\x0C/\x81_a\n\xCAV[a\x0C=Wa\x0C=\x813a\x0EGV[\x87\x86\x14\x15\x80a\x0CLWP\x87\x84\x14\x15[\x15a\x0C~W`@Q`\x01bO\xCD\xEF`\xE0\x1B\x03\x19\x81R`\x04\x81\x01\x89\x90R`$\x81\x01\x85\x90R`D\x81\x01\x87\x90R`d\x01a\x08mV[_a\x0C\x8F\x8A\x8A\x8A\x8A\x8A\x8A\x8A\x8Aa\n\xF2V[\x90Pa\x0C\x9B\x81\x85a\x0E\x84V[_[\x89\x81\x10\x15a\roW_\x8B\x8B\x83\x81\x81\x10a\x0C\xB8Wa\x0C\xB8a\x17\x90V[\x90P` \x02\x01` \x81\x01\x90a\x0C\xCD\x91\x90a\x17\xA4V[\x90P_\x8A\x8A\x84\x81\x81\x10a\x0C\xE2Wa\x0C\xE2a\x17\x90V[\x90P` \x02\x015\x90P6_\x8A\x8A\x86\x81\x81\x10a\x0C\xFFWa\x0C\xFFa\x17\x90V[\x90P` \x02\x81\x01\x90a\r\x11\x91\x90a\x17\xBDV[\x91P\x91Pa\r!\x84\x84\x84\x84a\x0E\xD2V[\x84\x86\x7F\xC2a~\xFAi\xBA\xB6g\x82\xFA!\x95CqC8H\x9CN\x9E\x17\x82qV\n\x91\xB8,?a+X\x86\x86\x86\x86`@Qa\rX\x94\x93\x92\x91\x90a\x17iV[`@Q\x80\x91\x03\x90\xA3PPPP\x80`\x01\x01\x90Pa\x0C\x9DV[Pa\ry\x81a\x0FFV[PPPPPPPPPPV[a\r\x8F\x813a\x0EGV[PV[a\r\x9B\x82a\x07\xA4V[\x15a\r\xCCW\x81a\r\xAA_a\x10iV[`@Qc^\xAD\x8E\xB5`\xE0\x1B\x81R`\x04\x81\x01\x92\x90\x92R`$\x82\x01R`D\x01a\x08mV[_a\r\xD6`\x02T\x90V[\x90P\x80\x82\x10\x15a\x0E\x03W`@QcT3f\t`\xE0\x1B\x81R`\x04\x81\x01\x83\x90R`$\x81\x01\x82\x90R`D\x01a\x08mV[a\x0E\r\x82Ba\x192V[_\x93\x84R`\x01` R`@\x90\x93 \x92\x90\x92UPPV[_`\x01`\x01`\xE0\x1B\x03\x19\x82\x16c\x02q\x18\x97`\xE5\x1B\x14\x80a\x06\x95WPa\x06\x95\x82a\x10\x8BV[a\x0EQ\x82\x82a\n\xCAV[a\x0E\x80W`@Qc\xE2Q}?`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x82\x16`\x04\x82\x01R`$\x81\x01\x83\x90R`D\x01a\x08mV[PPV[a\x0E\x8D\x82a\x07MV[a\x0E\x9CW\x81a\r\xAA`\x02a\x10iV[\x80\x15\x80\x15\x90a\x0E\xB1WPa\x0E\xAF\x81a\x07rV[\x15[\x15a\x0E\x80W`@Qc\x12\x154\xC3`\xE3\x1B\x81R`\x04\x81\x01\x82\x90R`$\x01a\x08mV[__\x85`\x01`\x01`\xA0\x1B\x03\x16\x85\x85\x85`@Qa\x0E\xEF\x92\x91\x90a\x19QV[_`@Q\x80\x83\x03\x81\x85\x87Z\xF1\x92PPP=\x80_\x81\x14a\x0F)W`@Q\x91P`\x1F\x19`?=\x01\x16\x82\x01`@R=\x82R=_` \x84\x01>a\x0F.V[``\x91P[P\x91P\x91Pa\x0F=\x82\x82a\x10\xBFV[PPPPPPPV[a\x0FO\x81a\x07MV[a\x0F^W\x80a\r\xAA`\x02a\x10iV[_\x90\x81R`\x01` \x81\x90R`@\x90\x91 UV[_a\x0F|\x83\x83a\n\xCAV[a\x0F\xF9W_\x83\x81R` \x81\x81R`@\x80\x83 `\x01`\x01`\xA0\x1B\x03\x86\x16\x84R\x90\x91R\x90 \x80T`\xFF\x19\x16`\x01\x17\x90Ua\x0F\xB13\x90V[`\x01`\x01`\xA0\x1B\x03\x16\x82`\x01`\x01`\xA0\x1B\x03\x16\x84\x7F/\x87\x88\x11~~\xFF\x1D\x82\xE9&\xECyI\x01\xD1|x\x02JP'\t@0E@\xA73eo\r`@Q`@Q\x80\x91\x03\x90\xA4P`\x01a\x06\x95V[P_a\x06\x95V[_a\x10\x0B\x83\x83a\n\xCAV[\x15a\x0F\xF9W_\x83\x81R` \x81\x81R`@\x80\x83 `\x01`\x01`\xA0\x1B\x03\x86\x16\x80\x85R\x92R\x80\x83 \x80T`\xFF\x19\x16\x90UQ3\x92\x86\x91\x7F\xF69\x1F\\2\xD9\xC6\x9D*G\xEAg\x0BD)t\xB595\xD1\xED\xC7\xFDd\xEB!\xE0G\xA89\x17\x1B\x91\x90\xA4P`\x01a\x06\x95V[_\x81`\x03\x81\x11\x15a\x10|Wa\x10|a\x13\xB1V[`\x01`\xFF\x91\x90\x91\x16\x1B\x92\x91PPV[_`\x01`\x01`\xE0\x1B\x03\x19\x82\x16cye\xDB\x0B`\xE0\x1B\x14\x80a\x06\x95WPc\x01\xFF\xC9\xA7`\xE0\x1B`\x01`\x01`\xE0\x1B\x03\x19\x83\x16\x14a\x06\x95V[``\x82a\x10\xD4Wa\x10\xCF\x82a\x10\xDBV[a\x06\x95V[P\x80a\x06\x95V[\x80Q\x15a\x10\xEBW\x80Q\x80\x82` \x01\xFD[`@Qc\n\x12\xF5!`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x11\x1AW__\xFD[\x91\x90PV[__\x83`\x1F\x84\x01\x12a\x11/W__\xFD[P\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x11EW__\xFD[` \x83\x01\x91P\x83` \x82\x85\x01\x01\x11\x15a\x11\\W__\xFD[\x92P\x92\x90PV[_______`\xC0\x88\x8A\x03\x12\x15a\x11yW__\xFD[a\x11\x82\x88a\x11\x04V[\x96P` \x88\x015\x95P`@\x88\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x11\xA3W__\xFD[a\x11\xAF\x8A\x82\x8B\x01a\x11\x1FV[\x98\x9B\x97\x9AP\x98``\x81\x015\x97`\x80\x82\x015\x97P`\xA0\x90\x91\x015\x95P\x93PPPPV[_` \x82\x84\x03\x12\x15a\x11\xE1W__\xFD[\x815`\x01`\x01`\xE0\x1B\x03\x19\x81\x16\x81\x14a\x08>W__\xFD[______`\xA0\x87\x89\x03\x12\x15a\x12\rW__\xFD[a\x12\x16\x87a\x11\x04V[\x95P` \x87\x015\x94P`@\x87\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x127W__\xFD[a\x12C\x89\x82\x8A\x01a\x11\x1FV[\x97\x9A\x96\x99P\x97``\x81\x015\x96`\x80\x90\x91\x015\x95P\x93PPPPV[_` \x82\x84\x03\x12\x15a\x12nW__\xFD[P5\x91\x90PV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x12\xB1Wa\x12\xB1a\x12uV[`@R\x91\x90PV[_\x82`\x1F\x83\x01\x12a\x12\xC8W__\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x12\xE1Wa\x12\xE1a\x12uV[a\x12\xF4`\x1F\x82\x01`\x1F\x19\x16` \x01a\x12\x89V[\x81\x81R\x84` \x83\x86\x01\x01\x11\x15a\x13\x08W__\xFD[\x81` \x85\x01` \x83\x017_\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[____`\x80\x85\x87\x03\x12\x15a\x137W__\xFD[a\x13@\x85a\x11\x04V[\x93Pa\x13N` \x86\x01a\x11\x04V[\x92P`@\x85\x015\x91P``\x85\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x13oW__\xFD[a\x13{\x87\x82\x88\x01a\x12\xB9V[\x91PP\x92\x95\x91\x94P\x92PV[__`@\x83\x85\x03\x12\x15a\x13\x98W__\xFD[\x825\x91Pa\x13\xA8` \x84\x01a\x11\x04V[\x90P\x92P\x92\x90PV[cNH{q`\xE0\x1B_R`!`\x04R`$_\xFD[` \x81\x01`\x04\x83\x10a\x13\xE5WcNH{q`\xE0\x1B_R`!`\x04R`$_\xFD[\x91\x90R\x90V[__\x83`\x1F\x84\x01\x12a\x13\xFBW__\xFD[P\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x14\x11W__\xFD[` \x83\x01\x91P\x83` \x82`\x05\x1B\x85\x01\x01\x11\x15a\x11\\W__\xFD[_________`\xC0\x8A\x8C\x03\x12\x15a\x14CW__\xFD[\x895`\x01`\x01`@\x1B\x03\x81\x11\x15a\x14XW__\xFD[a\x14d\x8C\x82\x8D\x01a\x13\xEBV[\x90\x9AP\x98PP` \x8A\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x14\x82W__\xFD[a\x14\x8E\x8C\x82\x8D\x01a\x13\xEBV[\x90\x98P\x96PP`@\x8A\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x14\xACW__\xFD[a\x14\xB8\x8C\x82\x8D\x01a\x13\xEBV[\x9A\x9D\x99\x9CP\x97\x9A\x96\x99\x97\x98``\x88\x015\x97`\x80\x81\x015\x97P`\xA0\x015\x95P\x93PPPPV[________`\xA0\x89\x8B\x03\x12\x15a\x14\xF4W__\xFD[\x885`\x01`\x01`@\x1B\x03\x81\x11\x15a\x15\tW__\xFD[a\x15\x15\x8B\x82\x8C\x01a\x13\xEBV[\x90\x99P\x97PP` \x89\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x153W__\xFD[a\x15?\x8B\x82\x8C\x01a\x13\xEBV[\x90\x97P\x95PP`@\x89\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x15]W__\xFD[a\x15i\x8B\x82\x8C\x01a\x13\xEBV[\x99\x9C\x98\x9BP\x96\x99\x95\x98\x96\x97``\x87\x015\x96`\x80\x015\x95P\x93PPPPV[_\x82`\x1F\x83\x01\x12a\x15\x96W__\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x15\xAFWa\x15\xAFa\x12uV[\x80`\x05\x1Ba\x15\xBF` \x82\x01a\x12\x89V[\x91\x82R` \x81\x85\x01\x81\x01\x92\x90\x81\x01\x90\x86\x84\x11\x15a\x15\xDAW__\xFD[` \x86\x01\x92P[\x83\x83\x10\x15a\x15\xFCW\x825\x82R` \x92\x83\x01\x92\x90\x91\x01\x90a\x15\xE1V[\x96\x95PPPPPPV[_____`\xA0\x86\x88\x03\x12\x15a\x16\x1AW__\xFD[a\x16#\x86a\x11\x04V[\x94Pa\x161` \x87\x01a\x11\x04V[\x93P`@\x86\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x16KW__\xFD[a\x16W\x88\x82\x89\x01a\x15\x87V[\x93PP``\x86\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x16rW__\xFD[a\x16~\x88\x82\x89\x01a\x15\x87V[\x92PP`\x80\x86\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x16\x99W__\xFD[a\x16\xA5\x88\x82\x89\x01a\x12\xB9V[\x91PP\x92\x95P\x92\x95\x90\x93PV[_____`\xA0\x86\x88\x03\x12\x15a\x16\xC6W__\xFD[a\x16\xCF\x86a\x11\x04V[\x94Pa\x16\xDD` \x87\x01a\x11\x04V[\x93P`@\x86\x015\x92P``\x86\x015\x91P`\x80\x86\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x16\x99W__\xFD[\x81\x83R\x81\x81` \x85\x017P_\x82\x82\x01` \x90\x81\x01\x91\x90\x91R`\x1F\x90\x91\x01`\x1F\x19\x16\x90\x91\x01\x01\x90V[`\x01\x80`\xA0\x1B\x03\x87\x16\x81R\x85` \x82\x01R`\xA0`@\x82\x01R_a\x17T`\xA0\x83\x01\x86\x88a\x17\x05V[``\x83\x01\x94\x90\x94RP`\x80\x01R\x94\x93PPPPV[`\x01\x80`\xA0\x1B\x03\x85\x16\x81R\x83` \x82\x01R```@\x82\x01R_a\x15\xFC``\x83\x01\x84\x86a\x17\x05V[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[_` \x82\x84\x03\x12\x15a\x17\xB4W__\xFD[a\x08>\x82a\x11\x04V[__\x835`\x1E\x19\x846\x03\x01\x81\x12a\x17\xD2W__\xFD[\x83\x01\x805\x91P`\x01`\x01`@\x1B\x03\x82\x11\x15a\x17\xEBW__\xFD[` \x01\x91P6\x81\x90\x03\x82\x13\x15a\x11\\W__\xFD[_\x83\x83\x85R` \x85\x01\x94P` \x84`\x05\x1B\x82\x01\x01\x83_[\x86\x81\x10\x15a\x18\x87W\x83\x83\x03`\x1F\x19\x01\x88R\x8156\x87\x90\x03`\x1E\x19\x01\x81\x12a\x18;W__\xFD[\x86\x01` \x81\x01\x905`\x01`\x01`@\x1B\x03\x81\x11\x15a\x18VW__\xFD[\x806\x03\x82\x13\x15a\x18dW__\xFD[a\x18o\x85\x82\x84a\x17\x05V[` \x9A\x8B\x01\x9A\x90\x95P\x93\x90\x93\x01\x92PP`\x01\x01a\x18\x16V[P\x90\x96\x95PPPPPPV[`\xA0\x80\x82R\x81\x01\x88\x90R_\x89`\xC0\x83\x01\x82[\x8B\x81\x10\x15a\x18\xD3W`\x01`\x01`\xA0\x1B\x03a\x18\xBE\x84a\x11\x04V[\x16\x82R` \x92\x83\x01\x92\x90\x91\x01\x90`\x01\x01a\x18\xA5V[P\x83\x81\x03` \x85\x01R\x88\x81R`\x01`\x01`\xFB\x1B\x03\x89\x11\x15a\x18\xF2W__\xFD[\x88`\x05\x1B\x91P\x81\x8A` \x83\x017\x01\x82\x81\x03` \x90\x81\x01`@\x85\x01Ra\x19\x1A\x90\x82\x01\x87\x89a\x17\xFFV[``\x84\x01\x95\x90\x95RPP`\x80\x01R\x96\x95PPPPPPV[\x80\x82\x01\x80\x82\x11\x15a\x06\x95WcNH{q`\xE0\x1B_R`\x11`\x04R`$_\xFD[\x81\x83\x827_\x91\x01\x90\x81R\x91\x90PV\xFE\xA1dsolcC\0\x08\x1C\0\n",
    );
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `AccessControlBadConfirmation()` and selector `0x6697b232`.
    ```solidity
    error AccessControlBadConfirmation();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct AccessControlBadConfirmation {}
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
        impl ::core::convert::From<AccessControlBadConfirmation> for UnderlyingRustTuple<'_> {
            fn from(value: AccessControlBadConfirmation) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for AccessControlBadConfirmation {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for AccessControlBadConfirmation {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "AccessControlBadConfirmation()";
            const SELECTOR: [u8; 4] = [102u8, 151u8, 178u8, 50u8];
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
    /**Custom error with signature `AccessControlUnauthorizedAccount(address,bytes32)` and selector `0xe2517d3f`.
    ```solidity
    error AccessControlUnauthorizedAccount(address account, bytes32 neededRole);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct AccessControlUnauthorizedAccount {
        #[allow(missing_docs)]
        pub account: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub neededRole: alloy::sol_types::private::FixedBytes<32>,
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
            alloy::sol_types::sol_data::Address,
            alloy::sol_types::sol_data::FixedBytes<32>,
        );
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (
            alloy::sol_types::private::Address,
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
        impl ::core::convert::From<AccessControlUnauthorizedAccount> for UnderlyingRustTuple<'_> {
            fn from(value: AccessControlUnauthorizedAccount) -> Self {
                (value.account, value.neededRole)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for AccessControlUnauthorizedAccount {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {
                    account: tuple.0,
                    neededRole: tuple.1,
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for AccessControlUnauthorizedAccount {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "AccessControlUnauthorizedAccount(address,bytes32)";
            const SELECTOR: [u8; 4] = [226u8, 81u8, 125u8, 63u8];
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
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.neededRole),
                )
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
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
    /**Custom error with signature `TimelockInsufficientDelay(uint256,uint256)` and selector `0x54336609`.
    ```solidity
    error TimelockInsufficientDelay(uint256 delay, uint256 minDelay);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct TimelockInsufficientDelay {
        #[allow(missing_docs)]
        pub delay: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub minDelay: alloy::sol_types::private::primitives::aliases::U256,
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
        );
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (
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
        impl ::core::convert::From<TimelockInsufficientDelay> for UnderlyingRustTuple<'_> {
            fn from(value: TimelockInsufficientDelay) -> Self {
                (value.delay, value.minDelay)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for TimelockInsufficientDelay {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {
                    delay: tuple.0,
                    minDelay: tuple.1,
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for TimelockInsufficientDelay {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "TimelockInsufficientDelay(uint256,uint256)";
            const SELECTOR: [u8; 4] = [84u8, 51u8, 102u8, 9u8];
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
                        &self.delay,
                    ),
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.minDelay,
                    ),
                )
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `TimelockInvalidOperationLength(uint256,uint256,uint256)` and selector `0xffb03211`.
    ```solidity
    error TimelockInvalidOperationLength(uint256 targets, uint256 payloads, uint256 values);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct TimelockInvalidOperationLength {
        #[allow(missing_docs)]
        pub targets: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub payloads: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub values: alloy::sol_types::private::primitives::aliases::U256,
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
            alloy::sol_types::sol_data::Uint<256>,
        );
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (
            alloy::sol_types::private::primitives::aliases::U256,
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
        impl ::core::convert::From<TimelockInvalidOperationLength> for UnderlyingRustTuple<'_> {
            fn from(value: TimelockInvalidOperationLength) -> Self {
                (value.targets, value.payloads, value.values)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for TimelockInvalidOperationLength {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {
                    targets: tuple.0,
                    payloads: tuple.1,
                    values: tuple.2,
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for TimelockInvalidOperationLength {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str =
                "TimelockInvalidOperationLength(uint256,uint256,uint256)";
            const SELECTOR: [u8; 4] = [255u8, 176u8, 50u8, 17u8];
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
                        &self.targets,
                    ),
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.payloads,
                    ),
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.values,
                    ),
                )
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `TimelockUnauthorizedCaller(address)` and selector `0xe2850c59`.
    ```solidity
    error TimelockUnauthorizedCaller(address caller);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct TimelockUnauthorizedCaller {
        #[allow(missing_docs)]
        pub caller: alloy::sol_types::private::Address,
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<TimelockUnauthorizedCaller> for UnderlyingRustTuple<'_> {
            fn from(value: TimelockUnauthorizedCaller) -> Self {
                (value.caller,)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for TimelockUnauthorizedCaller {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self { caller: tuple.0 }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for TimelockUnauthorizedCaller {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "TimelockUnauthorizedCaller(address)";
            const SELECTOR: [u8; 4] = [226u8, 133u8, 12u8, 89u8];
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
                        &self.caller,
                    ),
                )
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `TimelockUnexecutedPredecessor(bytes32)` and selector `0x90a9a618`.
    ```solidity
    error TimelockUnexecutedPredecessor(bytes32 predecessorId);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct TimelockUnexecutedPredecessor {
        #[allow(missing_docs)]
        pub predecessorId: alloy::sol_types::private::FixedBytes<32>,
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<TimelockUnexecutedPredecessor> for UnderlyingRustTuple<'_> {
            fn from(value: TimelockUnexecutedPredecessor) -> Self {
                (value.predecessorId,)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for TimelockUnexecutedPredecessor {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {
                    predecessorId: tuple.0,
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for TimelockUnexecutedPredecessor {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "TimelockUnexecutedPredecessor(bytes32)";
            const SELECTOR: [u8; 4] = [144u8, 169u8, 166u8, 24u8];
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
                    > as alloy_sol_types::SolType>::tokenize(&self.predecessorId),
                )
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `TimelockUnexpectedOperationState(bytes32,bytes32)` and selector `0x5ead8eb5`.
    ```solidity
    error TimelockUnexpectedOperationState(bytes32 operationId, bytes32 expectedStates);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct TimelockUnexpectedOperationState {
        #[allow(missing_docs)]
        pub operationId: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub expectedStates: alloy::sol_types::private::FixedBytes<32>,
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
            alloy::sol_types::sol_data::FixedBytes<32>,
            alloy::sol_types::sol_data::FixedBytes<32>,
        );
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (
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
        impl ::core::convert::From<TimelockUnexpectedOperationState> for UnderlyingRustTuple<'_> {
            fn from(value: TimelockUnexpectedOperationState) -> Self {
                (value.operationId, value.expectedStates)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for TimelockUnexpectedOperationState {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {
                    operationId: tuple.0,
                    expectedStates: tuple.1,
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for TimelockUnexpectedOperationState {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "TimelockUnexpectedOperationState(bytes32,bytes32)";
            const SELECTOR: [u8; 4] = [94u8, 173u8, 142u8, 181u8];
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
                    > as alloy_sol_types::SolType>::tokenize(&self.operationId),
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.expectedStates),
                )
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Event with signature `CallExecuted(bytes32,uint256,address,uint256,bytes)` and selector `0xc2617efa69bab66782fa219543714338489c4e9e178271560a91b82c3f612b58`.
    ```solidity
    event CallExecuted(bytes32 indexed id, uint256 indexed index, address target, uint256 value, bytes data);
    ```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct CallExecuted {
        #[allow(missing_docs)]
        pub id: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub index: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub target: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub value: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub data: alloy::sol_types::private::Bytes,
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
        impl alloy_sol_types::SolEvent for CallExecuted {
            type DataTuple<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Bytes,
            );
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Uint<256>,
            );
            const SIGNATURE: &'static str = "CallExecuted(bytes32,uint256,address,uint256,bytes)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 =
                alloy_sol_types::private::B256::new([
                    194u8, 97u8, 126u8, 250u8, 105u8, 186u8, 182u8, 103u8, 130u8, 250u8, 33u8,
                    149u8, 67u8, 113u8, 67u8, 56u8, 72u8, 156u8, 78u8, 158u8, 23u8, 130u8, 113u8,
                    86u8, 10u8, 145u8, 184u8, 44u8, 63u8, 97u8, 43u8, 88u8,
                ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self {
                    id: topics.1,
                    index: topics.2,
                    target: data.0,
                    value: data.1,
                    data: data.2,
                }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(alloy_sol_types::Error::invalid_event_signature_hash(
                        Self::SIGNATURE,
                        topics.0,
                        Self::SIGNATURE_HASH,
                    ));
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                (
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.target,
                    ),
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.value,
                    ),
                    <alloy::sol_types::sol_data::Bytes as alloy_sol_types::SolType>::tokenize(
                        &self.data,
                    ),
                )
            }
            #[inline]
            fn topics(&self) -> <Self::TopicList as alloy_sol_types::SolType>::RustType {
                (
                    Self::SIGNATURE_HASH.into(),
                    self.id.clone(),
                    self.index.clone(),
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
                out[0usize] = alloy_sol_types::abi::token::WordToken(Self::SIGNATURE_HASH);
                out[1usize] = <alloy::sol_types::sol_data::FixedBytes<
                    32,
                > as alloy_sol_types::EventTopic>::encode_topic(&self.id);
                out[2usize] = <alloy::sol_types::sol_data::Uint<
                    256,
                > as alloy_sol_types::EventTopic>::encode_topic(&self.index);
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for CallExecuted {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&CallExecuted> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &CallExecuted) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Event with signature `CallSalt(bytes32,bytes32)` and selector `0x20fda5fd27a1ea7bf5b9567f143ac5470bb059374a27e8f67cb44f946f6d0387`.
    ```solidity
    event CallSalt(bytes32 indexed id, bytes32 salt);
    ```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct CallSalt {
        #[allow(missing_docs)]
        pub id: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub salt: alloy::sol_types::private::FixedBytes<32>,
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
        impl alloy_sol_types::SolEvent for CallSalt {
            type DataTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::FixedBytes<32>,
            );
            const SIGNATURE: &'static str = "CallSalt(bytes32,bytes32)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 =
                alloy_sol_types::private::B256::new([
                    32u8, 253u8, 165u8, 253u8, 39u8, 161u8, 234u8, 123u8, 245u8, 185u8, 86u8,
                    127u8, 20u8, 58u8, 197u8, 71u8, 11u8, 176u8, 89u8, 55u8, 74u8, 39u8, 232u8,
                    246u8, 124u8, 180u8, 79u8, 148u8, 111u8, 109u8, 3u8, 135u8,
                ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self {
                    id: topics.1,
                    salt: data.0,
                }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(alloy_sol_types::Error::invalid_event_signature_hash(
                        Self::SIGNATURE,
                        topics.0,
                        Self::SIGNATURE_HASH,
                    ));
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                (
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.salt),
                )
            }
            #[inline]
            fn topics(&self) -> <Self::TopicList as alloy_sol_types::SolType>::RustType {
                (Self::SIGNATURE_HASH.into(), self.id.clone())
            }
            #[inline]
            fn encode_topics_raw(
                &self,
                out: &mut [alloy_sol_types::abi::token::WordToken],
            ) -> alloy_sol_types::Result<()> {
                if out.len() < <Self::TopicList as alloy_sol_types::TopicList>::COUNT {
                    return Err(alloy_sol_types::Error::Overrun);
                }
                out[0usize] = alloy_sol_types::abi::token::WordToken(Self::SIGNATURE_HASH);
                out[1usize] = <alloy::sol_types::sol_data::FixedBytes<
                    32,
                > as alloy_sol_types::EventTopic>::encode_topic(&self.id);
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for CallSalt {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&CallSalt> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &CallSalt) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Event with signature `CallScheduled(bytes32,uint256,address,uint256,bytes,bytes32,uint256)` and selector `0x4cf4410cc57040e44862ef0f45f3dd5a5e02db8eb8add648d4b0e236f1d07dca`.
    ```solidity
    event CallScheduled(bytes32 indexed id, uint256 indexed index, address target, uint256 value, bytes data, bytes32 predecessor, uint256 delay);
    ```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct CallScheduled {
        #[allow(missing_docs)]
        pub id: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub index: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub target: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub value: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub data: alloy::sol_types::private::Bytes,
        #[allow(missing_docs)]
        pub predecessor: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub delay: alloy::sol_types::private::primitives::aliases::U256,
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
        impl alloy_sol_types::SolEvent for CallScheduled {
            type DataTuple<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Bytes,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Uint<256>,
            );
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Uint<256>,
            );
            const SIGNATURE: &'static str =
                "CallScheduled(bytes32,uint256,address,uint256,bytes,bytes32,uint256)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 =
                alloy_sol_types::private::B256::new([
                    76u8, 244u8, 65u8, 12u8, 197u8, 112u8, 64u8, 228u8, 72u8, 98u8, 239u8, 15u8,
                    69u8, 243u8, 221u8, 90u8, 94u8, 2u8, 219u8, 142u8, 184u8, 173u8, 214u8, 72u8,
                    212u8, 176u8, 226u8, 54u8, 241u8, 208u8, 125u8, 202u8,
                ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self {
                    id: topics.1,
                    index: topics.2,
                    target: data.0,
                    value: data.1,
                    data: data.2,
                    predecessor: data.3,
                    delay: data.4,
                }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(alloy_sol_types::Error::invalid_event_signature_hash(
                        Self::SIGNATURE,
                        topics.0,
                        Self::SIGNATURE_HASH,
                    ));
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                (
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.target,
                    ),
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::tokenize(&self.value),
                    <alloy::sol_types::sol_data::Bytes as alloy_sol_types::SolType>::tokenize(
                        &self.data,
                    ),
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.predecessor),
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::tokenize(&self.delay),
                )
            }
            #[inline]
            fn topics(&self) -> <Self::TopicList as alloy_sol_types::SolType>::RustType {
                (
                    Self::SIGNATURE_HASH.into(),
                    self.id.clone(),
                    self.index.clone(),
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
                out[0usize] = alloy_sol_types::abi::token::WordToken(Self::SIGNATURE_HASH);
                out[1usize] = <alloy::sol_types::sol_data::FixedBytes<
                    32,
                > as alloy_sol_types::EventTopic>::encode_topic(&self.id);
                out[2usize] = <alloy::sol_types::sol_data::Uint<
                    256,
                > as alloy_sol_types::EventTopic>::encode_topic(&self.index);
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for CallScheduled {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&CallScheduled> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &CallScheduled) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Event with signature `Cancelled(bytes32)` and selector `0xbaa1eb22f2a492ba1a5fea61b8df4d27c6c8b5f3971e63bb58fa14ff72eedb70`.
    ```solidity
    event Cancelled(bytes32 indexed id);
    ```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct Cancelled {
        #[allow(missing_docs)]
        pub id: alloy::sol_types::private::FixedBytes<32>,
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
        impl alloy_sol_types::SolEvent for Cancelled {
            type DataTuple<'a> = ();
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::FixedBytes<32>,
            );
            const SIGNATURE: &'static str = "Cancelled(bytes32)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 =
                alloy_sol_types::private::B256::new([
                    186u8, 161u8, 235u8, 34u8, 242u8, 164u8, 146u8, 186u8, 26u8, 95u8, 234u8, 97u8,
                    184u8, 223u8, 77u8, 39u8, 198u8, 200u8, 181u8, 243u8, 151u8, 30u8, 99u8, 187u8,
                    88u8, 250u8, 20u8, 255u8, 114u8, 238u8, 219u8, 112u8,
                ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self { id: topics.1 }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(alloy_sol_types::Error::invalid_event_signature_hash(
                        Self::SIGNATURE,
                        topics.0,
                        Self::SIGNATURE_HASH,
                    ));
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                ()
            }
            #[inline]
            fn topics(&self) -> <Self::TopicList as alloy_sol_types::SolType>::RustType {
                (Self::SIGNATURE_HASH.into(), self.id.clone())
            }
            #[inline]
            fn encode_topics_raw(
                &self,
                out: &mut [alloy_sol_types::abi::token::WordToken],
            ) -> alloy_sol_types::Result<()> {
                if out.len() < <Self::TopicList as alloy_sol_types::TopicList>::COUNT {
                    return Err(alloy_sol_types::Error::Overrun);
                }
                out[0usize] = alloy_sol_types::abi::token::WordToken(Self::SIGNATURE_HASH);
                out[1usize] = <alloy::sol_types::sol_data::FixedBytes<
                    32,
                > as alloy_sol_types::EventTopic>::encode_topic(&self.id);
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for Cancelled {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&Cancelled> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &Cancelled) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Event with signature `MinDelayChange(uint256,uint256)` and selector `0x11c24f4ead16507c69ac467fbd5e4eed5fb5c699626d2cc6d66421df253886d5`.
    ```solidity
    event MinDelayChange(uint256 oldDuration, uint256 newDuration);
    ```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct MinDelayChange {
        #[allow(missing_docs)]
        pub oldDuration: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub newDuration: alloy::sol_types::private::primitives::aliases::U256,
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
        impl alloy_sol_types::SolEvent for MinDelayChange {
            type DataTuple<'a> = (
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Uint<256>,
            );
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (alloy_sol_types::sol_data::FixedBytes<32>,);
            const SIGNATURE: &'static str = "MinDelayChange(uint256,uint256)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 =
                alloy_sol_types::private::B256::new([
                    17u8, 194u8, 79u8, 78u8, 173u8, 22u8, 80u8, 124u8, 105u8, 172u8, 70u8, 127u8,
                    189u8, 94u8, 78u8, 237u8, 95u8, 181u8, 198u8, 153u8, 98u8, 109u8, 44u8, 198u8,
                    214u8, 100u8, 33u8, 223u8, 37u8, 56u8, 134u8, 213u8,
                ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self {
                    oldDuration: data.0,
                    newDuration: data.1,
                }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(alloy_sol_types::Error::invalid_event_signature_hash(
                        Self::SIGNATURE,
                        topics.0,
                        Self::SIGNATURE_HASH,
                    ));
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.oldDuration,
                    ),
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.newDuration,
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
                out[0usize] = alloy_sol_types::abi::token::WordToken(Self::SIGNATURE_HASH);
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for MinDelayChange {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&MinDelayChange> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &MinDelayChange) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Event with signature `RoleAdminChanged(bytes32,bytes32,bytes32)` and selector `0xbd79b86ffe0ab8e8776151514217cd7cacd52c909f66475c3af44e129f0b00ff`.
    ```solidity
    event RoleAdminChanged(bytes32 indexed role, bytes32 indexed previousAdminRole, bytes32 indexed newAdminRole);
    ```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct RoleAdminChanged {
        #[allow(missing_docs)]
        pub role: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub previousAdminRole: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub newAdminRole: alloy::sol_types::private::FixedBytes<32>,
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
        impl alloy_sol_types::SolEvent for RoleAdminChanged {
            type DataTuple<'a> = ();
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::FixedBytes<32>,
            );
            const SIGNATURE: &'static str = "RoleAdminChanged(bytes32,bytes32,bytes32)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 =
                alloy_sol_types::private::B256::new([
                    189u8, 121u8, 184u8, 111u8, 254u8, 10u8, 184u8, 232u8, 119u8, 97u8, 81u8, 81u8,
                    66u8, 23u8, 205u8, 124u8, 172u8, 213u8, 44u8, 144u8, 159u8, 102u8, 71u8, 92u8,
                    58u8, 244u8, 78u8, 18u8, 159u8, 11u8, 0u8, 255u8,
                ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self {
                    role: topics.1,
                    previousAdminRole: topics.2,
                    newAdminRole: topics.3,
                }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(alloy_sol_types::Error::invalid_event_signature_hash(
                        Self::SIGNATURE,
                        topics.0,
                        Self::SIGNATURE_HASH,
                    ));
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
                    self.role.clone(),
                    self.previousAdminRole.clone(),
                    self.newAdminRole.clone(),
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
                out[0usize] = alloy_sol_types::abi::token::WordToken(Self::SIGNATURE_HASH);
                out[1usize] = <alloy::sol_types::sol_data::FixedBytes<
                    32,
                > as alloy_sol_types::EventTopic>::encode_topic(&self.role);
                out[2usize] = <alloy::sol_types::sol_data::FixedBytes<
                    32,
                > as alloy_sol_types::EventTopic>::encode_topic(&self.previousAdminRole);
                out[3usize] = <alloy::sol_types::sol_data::FixedBytes<
                    32,
                > as alloy_sol_types::EventTopic>::encode_topic(&self.newAdminRole);
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for RoleAdminChanged {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&RoleAdminChanged> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &RoleAdminChanged) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Event with signature `RoleGranted(bytes32,address,address)` and selector `0x2f8788117e7eff1d82e926ec794901d17c78024a50270940304540a733656f0d`.
    ```solidity
    event RoleGranted(bytes32 indexed role, address indexed account, address indexed sender);
    ```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct RoleGranted {
        #[allow(missing_docs)]
        pub role: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub account: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub sender: alloy::sol_types::private::Address,
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
        impl alloy_sol_types::SolEvent for RoleGranted {
            type DataTuple<'a> = ();
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Address,
            );
            const SIGNATURE: &'static str = "RoleGranted(bytes32,address,address)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 =
                alloy_sol_types::private::B256::new([
                    47u8, 135u8, 136u8, 17u8, 126u8, 126u8, 255u8, 29u8, 130u8, 233u8, 38u8, 236u8,
                    121u8, 73u8, 1u8, 209u8, 124u8, 120u8, 2u8, 74u8, 80u8, 39u8, 9u8, 64u8, 48u8,
                    69u8, 64u8, 167u8, 51u8, 101u8, 111u8, 13u8,
                ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self {
                    role: topics.1,
                    account: topics.2,
                    sender: topics.3,
                }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(alloy_sol_types::Error::invalid_event_signature_hash(
                        Self::SIGNATURE,
                        topics.0,
                        Self::SIGNATURE_HASH,
                    ));
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
                    self.role.clone(),
                    self.account.clone(),
                    self.sender.clone(),
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
                out[0usize] = alloy_sol_types::abi::token::WordToken(Self::SIGNATURE_HASH);
                out[1usize] = <alloy::sol_types::sol_data::FixedBytes<
                    32,
                > as alloy_sol_types::EventTopic>::encode_topic(&self.role);
                out[2usize] = <alloy::sol_types::sol_data::Address as alloy_sol_types::EventTopic>::encode_topic(
                    &self.account,
                );
                out[3usize] = <alloy::sol_types::sol_data::Address as alloy_sol_types::EventTopic>::encode_topic(
                    &self.sender,
                );
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for RoleGranted {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&RoleGranted> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &RoleGranted) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Event with signature `RoleRevoked(bytes32,address,address)` and selector `0xf6391f5c32d9c69d2a47ea670b442974b53935d1edc7fd64eb21e047a839171b`.
    ```solidity
    event RoleRevoked(bytes32 indexed role, address indexed account, address indexed sender);
    ```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct RoleRevoked {
        #[allow(missing_docs)]
        pub role: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub account: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub sender: alloy::sol_types::private::Address,
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
        impl alloy_sol_types::SolEvent for RoleRevoked {
            type DataTuple<'a> = ();
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Address,
            );
            const SIGNATURE: &'static str = "RoleRevoked(bytes32,address,address)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 =
                alloy_sol_types::private::B256::new([
                    246u8, 57u8, 31u8, 92u8, 50u8, 217u8, 198u8, 157u8, 42u8, 71u8, 234u8, 103u8,
                    11u8, 68u8, 41u8, 116u8, 181u8, 57u8, 53u8, 209u8, 237u8, 199u8, 253u8, 100u8,
                    235u8, 33u8, 224u8, 71u8, 168u8, 57u8, 23u8, 27u8,
                ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self {
                    role: topics.1,
                    account: topics.2,
                    sender: topics.3,
                }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(alloy_sol_types::Error::invalid_event_signature_hash(
                        Self::SIGNATURE,
                        topics.0,
                        Self::SIGNATURE_HASH,
                    ));
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
                    self.role.clone(),
                    self.account.clone(),
                    self.sender.clone(),
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
                out[0usize] = alloy_sol_types::abi::token::WordToken(Self::SIGNATURE_HASH);
                out[1usize] = <alloy::sol_types::sol_data::FixedBytes<
                    32,
                > as alloy_sol_types::EventTopic>::encode_topic(&self.role);
                out[2usize] = <alloy::sol_types::sol_data::Address as alloy_sol_types::EventTopic>::encode_topic(
                    &self.account,
                );
                out[3usize] = <alloy::sol_types::sol_data::Address as alloy_sol_types::EventTopic>::encode_topic(
                    &self.sender,
                );
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for RoleRevoked {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&RoleRevoked> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &RoleRevoked) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    /**Constructor`.
    ```solidity
    constructor(uint256 minDelay, address[] proposers, address[] executors, address admin);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct constructorCall {
        #[allow(missing_docs)]
        pub minDelay: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub proposers: alloy::sol_types::private::Vec<alloy::sol_types::private::Address>,
        #[allow(missing_docs)]
        pub executors: alloy::sol_types::private::Vec<alloy::sol_types::private::Address>,
        #[allow(missing_docs)]
        pub admin: alloy::sol_types::private::Address,
    }
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Array<alloy::sol_types::sol_data::Address>,
                alloy::sol_types::sol_data::Array<alloy::sol_types::sol_data::Address>,
                alloy::sol_types::sol_data::Address,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::primitives::aliases::U256,
                alloy::sol_types::private::Vec<alloy::sol_types::private::Address>,
                alloy::sol_types::private::Vec<alloy::sol_types::private::Address>,
                alloy::sol_types::private::Address,
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
            impl ::core::convert::From<constructorCall> for UnderlyingRustTuple<'_> {
                fn from(value: constructorCall) -> Self {
                    (
                        value.minDelay,
                        value.proposers,
                        value.executors,
                        value.admin,
                    )
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for constructorCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        minDelay: tuple.0,
                        proposers: tuple.1,
                        executors: tuple.2,
                        admin: tuple.3,
                    }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolConstructor for constructorCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Array<alloy::sol_types::sol_data::Address>,
                alloy::sol_types::sol_data::Array<alloy::sol_types::sol_data::Address>,
                alloy::sol_types::sol_data::Address,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
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
                    > as alloy_sol_types::SolType>::tokenize(&self.minDelay),
                    <alloy::sol_types::sol_data::Array<
                        alloy::sol_types::sol_data::Address,
                    > as alloy_sol_types::SolType>::tokenize(&self.proposers),
                    <alloy::sol_types::sol_data::Array<
                        alloy::sol_types::sol_data::Address,
                    > as alloy_sol_types::SolType>::tokenize(&self.executors),
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.admin,
                    ),
                )
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `CANCELLER_ROLE()` and selector `0xb08e51c0`.
    ```solidity
    function CANCELLER_ROLE() external view returns (bytes32);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct CANCELLER_ROLECall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`CANCELLER_ROLE()`](CANCELLER_ROLECall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct CANCELLER_ROLEReturn {
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<CANCELLER_ROLECall> for UnderlyingRustTuple<'_> {
                fn from(value: CANCELLER_ROLECall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for CANCELLER_ROLECall {
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<CANCELLER_ROLEReturn> for UnderlyingRustTuple<'_> {
                fn from(value: CANCELLER_ROLEReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for CANCELLER_ROLEReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for CANCELLER_ROLECall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = CANCELLER_ROLEReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "CANCELLER_ROLE()";
            const SELECTOR: [u8; 4] = [176u8, 142u8, 81u8, 192u8];
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
    /**Function with signature `DEFAULT_ADMIN_ROLE()` and selector `0xa217fddf`.
    ```solidity
    function DEFAULT_ADMIN_ROLE() external view returns (bytes32);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct DEFAULT_ADMIN_ROLECall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`DEFAULT_ADMIN_ROLE()`](DEFAULT_ADMIN_ROLECall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct DEFAULT_ADMIN_ROLEReturn {
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<DEFAULT_ADMIN_ROLECall> for UnderlyingRustTuple<'_> {
                fn from(value: DEFAULT_ADMIN_ROLECall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for DEFAULT_ADMIN_ROLECall {
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<DEFAULT_ADMIN_ROLEReturn> for UnderlyingRustTuple<'_> {
                fn from(value: DEFAULT_ADMIN_ROLEReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for DEFAULT_ADMIN_ROLEReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for DEFAULT_ADMIN_ROLECall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = DEFAULT_ADMIN_ROLEReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "DEFAULT_ADMIN_ROLE()";
            const SELECTOR: [u8; 4] = [162u8, 23u8, 253u8, 223u8];
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
    /**Function with signature `EXECUTOR_ROLE()` and selector `0x07bd0265`.
    ```solidity
    function EXECUTOR_ROLE() external view returns (bytes32);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct EXECUTOR_ROLECall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`EXECUTOR_ROLE()`](EXECUTOR_ROLECall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct EXECUTOR_ROLEReturn {
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<EXECUTOR_ROLECall> for UnderlyingRustTuple<'_> {
                fn from(value: EXECUTOR_ROLECall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for EXECUTOR_ROLECall {
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<EXECUTOR_ROLEReturn> for UnderlyingRustTuple<'_> {
                fn from(value: EXECUTOR_ROLEReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for EXECUTOR_ROLEReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for EXECUTOR_ROLECall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = EXECUTOR_ROLEReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "EXECUTOR_ROLE()";
            const SELECTOR: [u8; 4] = [7u8, 189u8, 2u8, 101u8];
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
    /**Function with signature `PROPOSER_ROLE()` and selector `0x8f61f4f5`.
    ```solidity
    function PROPOSER_ROLE() external view returns (bytes32);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct PROPOSER_ROLECall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`PROPOSER_ROLE()`](PROPOSER_ROLECall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct PROPOSER_ROLEReturn {
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<PROPOSER_ROLECall> for UnderlyingRustTuple<'_> {
                fn from(value: PROPOSER_ROLECall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for PROPOSER_ROLECall {
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<PROPOSER_ROLEReturn> for UnderlyingRustTuple<'_> {
                fn from(value: PROPOSER_ROLEReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for PROPOSER_ROLEReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for PROPOSER_ROLECall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = PROPOSER_ROLEReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "PROPOSER_ROLE()";
            const SELECTOR: [u8; 4] = [143u8, 97u8, 244u8, 245u8];
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
    /**Function with signature `cancel(bytes32)` and selector `0xc4d252f5`.
    ```solidity
    function cancel(bytes32 id) external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct cancelCall {
        #[allow(missing_docs)]
        pub id: alloy::sol_types::private::FixedBytes<32>,
    }
    ///Container type for the return parameters of the [`cancel(bytes32)`](cancelCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct cancelReturn {}
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
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::FixedBytes<32>,);
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
            impl ::core::convert::From<cancelCall> for UnderlyingRustTuple<'_> {
                fn from(value: cancelCall) -> Self {
                    (value.id,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for cancelCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { id: tuple.0 }
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<cancelReturn> for UnderlyingRustTuple<'_> {
                fn from(value: cancelReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for cancelReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for cancelCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = cancelReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "cancel(bytes32)";
            const SELECTOR: [u8; 4] = [196u8, 210u8, 82u8, 245u8];
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
                    > as alloy_sol_types::SolType>::tokenize(&self.id),
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `execute(address,uint256,bytes,bytes32,bytes32)` and selector `0x134008d3`.
    ```solidity
    function execute(address target, uint256 value, bytes memory payload, bytes32 predecessor, bytes32 salt) external payable;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct executeCall {
        #[allow(missing_docs)]
        pub target: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub value: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub payload: alloy::sol_types::private::Bytes,
        #[allow(missing_docs)]
        pub predecessor: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub salt: alloy::sol_types::private::FixedBytes<32>,
    }
    ///Container type for the return parameters of the [`execute(address,uint256,bytes,bytes32,bytes32)`](executeCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct executeReturn {}
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
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Bytes,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::FixedBytes<32>,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::Address,
                alloy::sol_types::private::primitives::aliases::U256,
                alloy::sol_types::private::Bytes,
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
            impl ::core::convert::From<executeCall> for UnderlyingRustTuple<'_> {
                fn from(value: executeCall) -> Self {
                    (
                        value.target,
                        value.value,
                        value.payload,
                        value.predecessor,
                        value.salt,
                    )
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for executeCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        target: tuple.0,
                        value: tuple.1,
                        payload: tuple.2,
                        predecessor: tuple.3,
                        salt: tuple.4,
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<executeReturn> for UnderlyingRustTuple<'_> {
                fn from(value: executeReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for executeReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for executeCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Bytes,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::FixedBytes<32>,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = executeReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "execute(address,uint256,bytes,bytes32,bytes32)";
            const SELECTOR: [u8; 4] = [19u8, 64u8, 8u8, 211u8];
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
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::tokenize(&self.value),
                    <alloy::sol_types::sol_data::Bytes as alloy_sol_types::SolType>::tokenize(
                        &self.payload,
                    ),
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.predecessor),
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.salt),
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `executeBatch(address[],uint256[],bytes[],bytes32,bytes32)` and selector `0xe38335e5`.
    ```solidity
    function executeBatch(address[] memory targets, uint256[] memory values, bytes[] memory payloads, bytes32 predecessor, bytes32 salt) external payable;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct executeBatchCall {
        #[allow(missing_docs)]
        pub targets: alloy::sol_types::private::Vec<alloy::sol_types::private::Address>,
        #[allow(missing_docs)]
        pub values:
            alloy::sol_types::private::Vec<alloy::sol_types::private::primitives::aliases::U256>,
        #[allow(missing_docs)]
        pub payloads: alloy::sol_types::private::Vec<alloy::sol_types::private::Bytes>,
        #[allow(missing_docs)]
        pub predecessor: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub salt: alloy::sol_types::private::FixedBytes<32>,
    }
    ///Container type for the return parameters of the [`executeBatch(address[],uint256[],bytes[],bytes32,bytes32)`](executeBatchCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct executeBatchReturn {}
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
                alloy::sol_types::sol_data::Array<alloy::sol_types::sol_data::Address>,
                alloy::sol_types::sol_data::Array<alloy::sol_types::sol_data::Uint<256>>,
                alloy::sol_types::sol_data::Array<alloy::sol_types::sol_data::Bytes>,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::FixedBytes<32>,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::Vec<alloy::sol_types::private::Address>,
                alloy::sol_types::private::Vec<
                    alloy::sol_types::private::primitives::aliases::U256,
                >,
                alloy::sol_types::private::Vec<alloy::sol_types::private::Bytes>,
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
            impl ::core::convert::From<executeBatchCall> for UnderlyingRustTuple<'_> {
                fn from(value: executeBatchCall) -> Self {
                    (
                        value.targets,
                        value.values,
                        value.payloads,
                        value.predecessor,
                        value.salt,
                    )
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for executeBatchCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        targets: tuple.0,
                        values: tuple.1,
                        payloads: tuple.2,
                        predecessor: tuple.3,
                        salt: tuple.4,
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<executeBatchReturn> for UnderlyingRustTuple<'_> {
                fn from(value: executeBatchReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for executeBatchReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for executeBatchCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::Array<alloy::sol_types::sol_data::Address>,
                alloy::sol_types::sol_data::Array<alloy::sol_types::sol_data::Uint<256>>,
                alloy::sol_types::sol_data::Array<alloy::sol_types::sol_data::Bytes>,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::FixedBytes<32>,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = executeBatchReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str =
                "executeBatch(address[],uint256[],bytes[],bytes32,bytes32)";
            const SELECTOR: [u8; 4] = [227u8, 131u8, 53u8, 229u8];
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
                        alloy::sol_types::sol_data::Address,
                    > as alloy_sol_types::SolType>::tokenize(&self.targets),
                    <alloy::sol_types::sol_data::Array<
                        alloy::sol_types::sol_data::Uint<256>,
                    > as alloy_sol_types::SolType>::tokenize(&self.values),
                    <alloy::sol_types::sol_data::Array<
                        alloy::sol_types::sol_data::Bytes,
                    > as alloy_sol_types::SolType>::tokenize(&self.payloads),
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.predecessor),
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.salt),
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `getMinDelay()` and selector `0xf27a0c92`.
    ```solidity
    function getMinDelay() external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct getMinDelayCall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`getMinDelay()`](getMinDelayCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct getMinDelayReturn {
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
            impl ::core::convert::From<getMinDelayCall> for UnderlyingRustTuple<'_> {
                fn from(value: getMinDelayCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for getMinDelayCall {
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
            impl ::core::convert::From<getMinDelayReturn> for UnderlyingRustTuple<'_> {
                fn from(value: getMinDelayReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for getMinDelayReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for getMinDelayCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = getMinDelayReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "getMinDelay()";
            const SELECTOR: [u8; 4] = [242u8, 122u8, 12u8, 146u8];
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
    /**Function with signature `getOperationState(bytes32)` and selector `0x7958004c`.
    ```solidity
    function getOperationState(bytes32 id) external view returns (TimelockController.OperationState);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct getOperationStateCall {
        #[allow(missing_docs)]
        pub id: alloy::sol_types::private::FixedBytes<32>,
    }
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`getOperationState(bytes32)`](getOperationStateCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct getOperationStateReturn {
        #[allow(missing_docs)]
        pub _0: <TimelockController::OperationState as alloy::sol_types::SolType>::RustType,
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
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::FixedBytes<32>,);
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
            impl ::core::convert::From<getOperationStateCall> for UnderlyingRustTuple<'_> {
                fn from(value: getOperationStateCall) -> Self {
                    (value.id,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for getOperationStateCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { id: tuple.0 }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (TimelockController::OperationState,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> =
                (<TimelockController::OperationState as alloy::sol_types::SolType>::RustType,);
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
            impl ::core::convert::From<getOperationStateReturn> for UnderlyingRustTuple<'_> {
                fn from(value: getOperationStateReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for getOperationStateReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for getOperationStateCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = getOperationStateReturn;
            type ReturnTuple<'a> = (TimelockController::OperationState,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "getOperationState(bytes32)";
            const SELECTOR: [u8; 4] = [121u8, 88u8, 0u8, 76u8];
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
                    > as alloy_sol_types::SolType>::tokenize(&self.id),
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `getRoleAdmin(bytes32)` and selector `0x248a9ca3`.
    ```solidity
    function getRoleAdmin(bytes32 role) external view returns (bytes32);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct getRoleAdminCall {
        #[allow(missing_docs)]
        pub role: alloy::sol_types::private::FixedBytes<32>,
    }
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`getRoleAdmin(bytes32)`](getRoleAdminCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct getRoleAdminReturn {
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
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::FixedBytes<32>,);
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
            impl ::core::convert::From<getRoleAdminCall> for UnderlyingRustTuple<'_> {
                fn from(value: getRoleAdminCall) -> Self {
                    (value.role,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for getRoleAdminCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { role: tuple.0 }
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<getRoleAdminReturn> for UnderlyingRustTuple<'_> {
                fn from(value: getRoleAdminReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for getRoleAdminReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for getRoleAdminCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = getRoleAdminReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "getRoleAdmin(bytes32)";
            const SELECTOR: [u8; 4] = [36u8, 138u8, 156u8, 163u8];
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
                    > as alloy_sol_types::SolType>::tokenize(&self.role),
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `getTimestamp(bytes32)` and selector `0xd45c4435`.
    ```solidity
    function getTimestamp(bytes32 id) external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct getTimestampCall {
        #[allow(missing_docs)]
        pub id: alloy::sol_types::private::FixedBytes<32>,
    }
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`getTimestamp(bytes32)`](getTimestampCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct getTimestampReturn {
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
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::FixedBytes<32>,);
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
            impl ::core::convert::From<getTimestampCall> for UnderlyingRustTuple<'_> {
                fn from(value: getTimestampCall) -> Self {
                    (value.id,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for getTimestampCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { id: tuple.0 }
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
            impl ::core::convert::From<getTimestampReturn> for UnderlyingRustTuple<'_> {
                fn from(value: getTimestampReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for getTimestampReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for getTimestampCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = getTimestampReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "getTimestamp(bytes32)";
            const SELECTOR: [u8; 4] = [212u8, 92u8, 68u8, 53u8];
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
                    > as alloy_sol_types::SolType>::tokenize(&self.id),
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `grantRole(bytes32,address)` and selector `0x2f2ff15d`.
    ```solidity
    function grantRole(bytes32 role, address account) external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct grantRoleCall {
        #[allow(missing_docs)]
        pub role: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub account: alloy::sol_types::private::Address,
    }
    ///Container type for the return parameters of the [`grantRole(bytes32,address)`](grantRoleCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct grantRoleReturn {}
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
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::FixedBytes<32>,
                alloy::sol_types::private::Address,
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
            impl ::core::convert::From<grantRoleCall> for UnderlyingRustTuple<'_> {
                fn from(value: grantRoleCall) -> Self {
                    (value.role, value.account)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for grantRoleCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        role: tuple.0,
                        account: tuple.1,
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<grantRoleReturn> for UnderlyingRustTuple<'_> {
                fn from(value: grantRoleReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for grantRoleReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for grantRoleCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = grantRoleReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "grantRole(bytes32,address)";
            const SELECTOR: [u8; 4] = [47u8, 47u8, 241u8, 93u8];
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
                    > as alloy_sol_types::SolType>::tokenize(&self.role),
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.account,
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `hasRole(bytes32,address)` and selector `0x91d14854`.
    ```solidity
    function hasRole(bytes32 role, address account) external view returns (bool);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct hasRoleCall {
        #[allow(missing_docs)]
        pub role: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub account: alloy::sol_types::private::Address,
    }
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`hasRole(bytes32,address)`](hasRoleCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct hasRoleReturn {
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
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::FixedBytes<32>,
                alloy::sol_types::private::Address,
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
            impl ::core::convert::From<hasRoleCall> for UnderlyingRustTuple<'_> {
                fn from(value: hasRoleCall) -> Self {
                    (value.role, value.account)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for hasRoleCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        role: tuple.0,
                        account: tuple.1,
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
            impl ::core::convert::From<hasRoleReturn> for UnderlyingRustTuple<'_> {
                fn from(value: hasRoleReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for hasRoleReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for hasRoleCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = hasRoleReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "hasRole(bytes32,address)";
            const SELECTOR: [u8; 4] = [145u8, 209u8, 72u8, 84u8];
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
                    > as alloy_sol_types::SolType>::tokenize(&self.role),
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.account,
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `hashOperation(address,uint256,bytes,bytes32,bytes32)` and selector `0x8065657f`.
    ```solidity
    function hashOperation(address target, uint256 value, bytes memory data, bytes32 predecessor, bytes32 salt) external pure returns (bytes32);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct hashOperationCall {
        #[allow(missing_docs)]
        pub target: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub value: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub data: alloy::sol_types::private::Bytes,
        #[allow(missing_docs)]
        pub predecessor: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub salt: alloy::sol_types::private::FixedBytes<32>,
    }
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`hashOperation(address,uint256,bytes,bytes32,bytes32)`](hashOperationCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct hashOperationReturn {
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
            type UnderlyingSolTuple<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Bytes,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::FixedBytes<32>,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::Address,
                alloy::sol_types::private::primitives::aliases::U256,
                alloy::sol_types::private::Bytes,
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
            impl ::core::convert::From<hashOperationCall> for UnderlyingRustTuple<'_> {
                fn from(value: hashOperationCall) -> Self {
                    (
                        value.target,
                        value.value,
                        value.data,
                        value.predecessor,
                        value.salt,
                    )
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for hashOperationCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        target: tuple.0,
                        value: tuple.1,
                        data: tuple.2,
                        predecessor: tuple.3,
                        salt: tuple.4,
                    }
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<hashOperationReturn> for UnderlyingRustTuple<'_> {
                fn from(value: hashOperationReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for hashOperationReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for hashOperationCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Bytes,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::FixedBytes<32>,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = hashOperationReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "hashOperation(address,uint256,bytes,bytes32,bytes32)";
            const SELECTOR: [u8; 4] = [128u8, 101u8, 101u8, 127u8];
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
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::tokenize(&self.value),
                    <alloy::sol_types::sol_data::Bytes as alloy_sol_types::SolType>::tokenize(
                        &self.data,
                    ),
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.predecessor),
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.salt),
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `hashOperationBatch(address[],uint256[],bytes[],bytes32,bytes32)` and selector `0xb1c5f427`.
    ```solidity
    function hashOperationBatch(address[] memory targets, uint256[] memory values, bytes[] memory payloads, bytes32 predecessor, bytes32 salt) external pure returns (bytes32);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct hashOperationBatchCall {
        #[allow(missing_docs)]
        pub targets: alloy::sol_types::private::Vec<alloy::sol_types::private::Address>,
        #[allow(missing_docs)]
        pub values:
            alloy::sol_types::private::Vec<alloy::sol_types::private::primitives::aliases::U256>,
        #[allow(missing_docs)]
        pub payloads: alloy::sol_types::private::Vec<alloy::sol_types::private::Bytes>,
        #[allow(missing_docs)]
        pub predecessor: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub salt: alloy::sol_types::private::FixedBytes<32>,
    }
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`hashOperationBatch(address[],uint256[],bytes[],bytes32,bytes32)`](hashOperationBatchCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct hashOperationBatchReturn {
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
            type UnderlyingSolTuple<'a> = (
                alloy::sol_types::sol_data::Array<alloy::sol_types::sol_data::Address>,
                alloy::sol_types::sol_data::Array<alloy::sol_types::sol_data::Uint<256>>,
                alloy::sol_types::sol_data::Array<alloy::sol_types::sol_data::Bytes>,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::FixedBytes<32>,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::Vec<alloy::sol_types::private::Address>,
                alloy::sol_types::private::Vec<
                    alloy::sol_types::private::primitives::aliases::U256,
                >,
                alloy::sol_types::private::Vec<alloy::sol_types::private::Bytes>,
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
            impl ::core::convert::From<hashOperationBatchCall> for UnderlyingRustTuple<'_> {
                fn from(value: hashOperationBatchCall) -> Self {
                    (
                        value.targets,
                        value.values,
                        value.payloads,
                        value.predecessor,
                        value.salt,
                    )
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for hashOperationBatchCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        targets: tuple.0,
                        values: tuple.1,
                        payloads: tuple.2,
                        predecessor: tuple.3,
                        salt: tuple.4,
                    }
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<hashOperationBatchReturn> for UnderlyingRustTuple<'_> {
                fn from(value: hashOperationBatchReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for hashOperationBatchReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for hashOperationBatchCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::Array<alloy::sol_types::sol_data::Address>,
                alloy::sol_types::sol_data::Array<alloy::sol_types::sol_data::Uint<256>>,
                alloy::sol_types::sol_data::Array<alloy::sol_types::sol_data::Bytes>,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::FixedBytes<32>,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = hashOperationBatchReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str =
                "hashOperationBatch(address[],uint256[],bytes[],bytes32,bytes32)";
            const SELECTOR: [u8; 4] = [177u8, 197u8, 244u8, 39u8];
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
                        alloy::sol_types::sol_data::Address,
                    > as alloy_sol_types::SolType>::tokenize(&self.targets),
                    <alloy::sol_types::sol_data::Array<
                        alloy::sol_types::sol_data::Uint<256>,
                    > as alloy_sol_types::SolType>::tokenize(&self.values),
                    <alloy::sol_types::sol_data::Array<
                        alloy::sol_types::sol_data::Bytes,
                    > as alloy_sol_types::SolType>::tokenize(&self.payloads),
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.predecessor),
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.salt),
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `isOperation(bytes32)` and selector `0x31d50750`.
    ```solidity
    function isOperation(bytes32 id) external view returns (bool);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct isOperationCall {
        #[allow(missing_docs)]
        pub id: alloy::sol_types::private::FixedBytes<32>,
    }
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`isOperation(bytes32)`](isOperationCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct isOperationReturn {
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
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::FixedBytes<32>,);
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
            impl ::core::convert::From<isOperationCall> for UnderlyingRustTuple<'_> {
                fn from(value: isOperationCall) -> Self {
                    (value.id,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for isOperationCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { id: tuple.0 }
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
            impl ::core::convert::From<isOperationReturn> for UnderlyingRustTuple<'_> {
                fn from(value: isOperationReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for isOperationReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for isOperationCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = isOperationReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "isOperation(bytes32)";
            const SELECTOR: [u8; 4] = [49u8, 213u8, 7u8, 80u8];
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
                    > as alloy_sol_types::SolType>::tokenize(&self.id),
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `isOperationDone(bytes32)` and selector `0x2ab0f529`.
    ```solidity
    function isOperationDone(bytes32 id) external view returns (bool);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct isOperationDoneCall {
        #[allow(missing_docs)]
        pub id: alloy::sol_types::private::FixedBytes<32>,
    }
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`isOperationDone(bytes32)`](isOperationDoneCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct isOperationDoneReturn {
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
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::FixedBytes<32>,);
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
            impl ::core::convert::From<isOperationDoneCall> for UnderlyingRustTuple<'_> {
                fn from(value: isOperationDoneCall) -> Self {
                    (value.id,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for isOperationDoneCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { id: tuple.0 }
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
            impl ::core::convert::From<isOperationDoneReturn> for UnderlyingRustTuple<'_> {
                fn from(value: isOperationDoneReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for isOperationDoneReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for isOperationDoneCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = isOperationDoneReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "isOperationDone(bytes32)";
            const SELECTOR: [u8; 4] = [42u8, 176u8, 245u8, 41u8];
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
                    > as alloy_sol_types::SolType>::tokenize(&self.id),
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `isOperationPending(bytes32)` and selector `0x584b153e`.
    ```solidity
    function isOperationPending(bytes32 id) external view returns (bool);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct isOperationPendingCall {
        #[allow(missing_docs)]
        pub id: alloy::sol_types::private::FixedBytes<32>,
    }
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`isOperationPending(bytes32)`](isOperationPendingCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct isOperationPendingReturn {
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
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::FixedBytes<32>,);
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
            impl ::core::convert::From<isOperationPendingCall> for UnderlyingRustTuple<'_> {
                fn from(value: isOperationPendingCall) -> Self {
                    (value.id,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for isOperationPendingCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { id: tuple.0 }
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
            impl ::core::convert::From<isOperationPendingReturn> for UnderlyingRustTuple<'_> {
                fn from(value: isOperationPendingReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for isOperationPendingReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for isOperationPendingCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = isOperationPendingReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "isOperationPending(bytes32)";
            const SELECTOR: [u8; 4] = [88u8, 75u8, 21u8, 62u8];
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
                    > as alloy_sol_types::SolType>::tokenize(&self.id),
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `isOperationReady(bytes32)` and selector `0x13bc9f20`.
    ```solidity
    function isOperationReady(bytes32 id) external view returns (bool);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct isOperationReadyCall {
        #[allow(missing_docs)]
        pub id: alloy::sol_types::private::FixedBytes<32>,
    }
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`isOperationReady(bytes32)`](isOperationReadyCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct isOperationReadyReturn {
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
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::FixedBytes<32>,);
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
            impl ::core::convert::From<isOperationReadyCall> for UnderlyingRustTuple<'_> {
                fn from(value: isOperationReadyCall) -> Self {
                    (value.id,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for isOperationReadyCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { id: tuple.0 }
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
            impl ::core::convert::From<isOperationReadyReturn> for UnderlyingRustTuple<'_> {
                fn from(value: isOperationReadyReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for isOperationReadyReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for isOperationReadyCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = isOperationReadyReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "isOperationReady(bytes32)";
            const SELECTOR: [u8; 4] = [19u8, 188u8, 159u8, 32u8];
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
                    > as alloy_sol_types::SolType>::tokenize(&self.id),
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `onERC1155BatchReceived(address,address,uint256[],uint256[],bytes)` and selector `0xbc197c81`.
    ```solidity
    function onERC1155BatchReceived(address, address, uint256[] memory, uint256[] memory, bytes memory) external returns (bytes4);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct onERC1155BatchReceivedCall {
        #[allow(missing_docs)]
        pub _0: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub _1: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub _2:
            alloy::sol_types::private::Vec<alloy::sol_types::private::primitives::aliases::U256>,
        #[allow(missing_docs)]
        pub _3:
            alloy::sol_types::private::Vec<alloy::sol_types::private::primitives::aliases::U256>,
        #[allow(missing_docs)]
        pub _4: alloy::sol_types::private::Bytes,
    }
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`onERC1155BatchReceived(address,address,uint256[],uint256[],bytes)`](onERC1155BatchReceivedCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct onERC1155BatchReceivedReturn {
        #[allow(missing_docs)]
        pub _0: alloy::sol_types::private::FixedBytes<4>,
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
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Array<alloy::sol_types::sol_data::Uint<256>>,
                alloy::sol_types::sol_data::Array<alloy::sol_types::sol_data::Uint<256>>,
                alloy::sol_types::sol_data::Bytes,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::Address,
                alloy::sol_types::private::Address,
                alloy::sol_types::private::Vec<
                    alloy::sol_types::private::primitives::aliases::U256,
                >,
                alloy::sol_types::private::Vec<
                    alloy::sol_types::private::primitives::aliases::U256,
                >,
                alloy::sol_types::private::Bytes,
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
            impl ::core::convert::From<onERC1155BatchReceivedCall> for UnderlyingRustTuple<'_> {
                fn from(value: onERC1155BatchReceivedCall) -> Self {
                    (value._0, value._1, value._2, value._3, value._4)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for onERC1155BatchReceivedCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        _0: tuple.0,
                        _1: tuple.1,
                        _2: tuple.2,
                        _3: tuple.3,
                        _4: tuple.4,
                    }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<4>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::FixedBytes<4>,);
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
            impl ::core::convert::From<onERC1155BatchReceivedReturn> for UnderlyingRustTuple<'_> {
                fn from(value: onERC1155BatchReceivedReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for onERC1155BatchReceivedReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for onERC1155BatchReceivedCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Array<alloy::sol_types::sol_data::Uint<256>>,
                alloy::sol_types::sol_data::Array<alloy::sol_types::sol_data::Uint<256>>,
                alloy::sol_types::sol_data::Bytes,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = onERC1155BatchReceivedReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<4>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str =
                "onERC1155BatchReceived(address,address,uint256[],uint256[],bytes)";
            const SELECTOR: [u8; 4] = [188u8, 25u8, 124u8, 129u8];
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
                        &self._0,
                    ),
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self._1,
                    ),
                    <alloy::sol_types::sol_data::Array<
                        alloy::sol_types::sol_data::Uint<256>,
                    > as alloy_sol_types::SolType>::tokenize(&self._2),
                    <alloy::sol_types::sol_data::Array<
                        alloy::sol_types::sol_data::Uint<256>,
                    > as alloy_sol_types::SolType>::tokenize(&self._3),
                    <alloy::sol_types::sol_data::Bytes as alloy_sol_types::SolType>::tokenize(
                        &self._4,
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `onERC1155Received(address,address,uint256,uint256,bytes)` and selector `0xf23a6e61`.
    ```solidity
    function onERC1155Received(address, address, uint256, uint256, bytes memory) external returns (bytes4);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct onERC1155ReceivedCall {
        #[allow(missing_docs)]
        pub _0: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub _1: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub _2: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub _3: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub _4: alloy::sol_types::private::Bytes,
    }
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`onERC1155Received(address,address,uint256,uint256,bytes)`](onERC1155ReceivedCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct onERC1155ReceivedReturn {
        #[allow(missing_docs)]
        pub _0: alloy::sol_types::private::FixedBytes<4>,
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
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Bytes,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::Address,
                alloy::sol_types::private::Address,
                alloy::sol_types::private::primitives::aliases::U256,
                alloy::sol_types::private::primitives::aliases::U256,
                alloy::sol_types::private::Bytes,
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
            impl ::core::convert::From<onERC1155ReceivedCall> for UnderlyingRustTuple<'_> {
                fn from(value: onERC1155ReceivedCall) -> Self {
                    (value._0, value._1, value._2, value._3, value._4)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for onERC1155ReceivedCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        _0: tuple.0,
                        _1: tuple.1,
                        _2: tuple.2,
                        _3: tuple.3,
                        _4: tuple.4,
                    }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<4>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::FixedBytes<4>,);
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
            impl ::core::convert::From<onERC1155ReceivedReturn> for UnderlyingRustTuple<'_> {
                fn from(value: onERC1155ReceivedReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for onERC1155ReceivedReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for onERC1155ReceivedCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Bytes,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = onERC1155ReceivedReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<4>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str =
                "onERC1155Received(address,address,uint256,uint256,bytes)";
            const SELECTOR: [u8; 4] = [242u8, 58u8, 110u8, 97u8];
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
                        &self._0,
                    ),
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self._1,
                    ),
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self._2,
                    ),
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self._3,
                    ),
                    <alloy::sol_types::sol_data::Bytes as alloy_sol_types::SolType>::tokenize(
                        &self._4,
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `onERC721Received(address,address,uint256,bytes)` and selector `0x150b7a02`.
    ```solidity
    function onERC721Received(address, address, uint256, bytes memory) external returns (bytes4);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct onERC721ReceivedCall {
        #[allow(missing_docs)]
        pub _0: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub _1: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub _2: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub _3: alloy::sol_types::private::Bytes,
    }
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`onERC721Received(address,address,uint256,bytes)`](onERC721ReceivedCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct onERC721ReceivedReturn {
        #[allow(missing_docs)]
        pub _0: alloy::sol_types::private::FixedBytes<4>,
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
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Bytes,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::Address,
                alloy::sol_types::private::Address,
                alloy::sol_types::private::primitives::aliases::U256,
                alloy::sol_types::private::Bytes,
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
            impl ::core::convert::From<onERC721ReceivedCall> for UnderlyingRustTuple<'_> {
                fn from(value: onERC721ReceivedCall) -> Self {
                    (value._0, value._1, value._2, value._3)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for onERC721ReceivedCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        _0: tuple.0,
                        _1: tuple.1,
                        _2: tuple.2,
                        _3: tuple.3,
                    }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<4>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::FixedBytes<4>,);
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
            impl ::core::convert::From<onERC721ReceivedReturn> for UnderlyingRustTuple<'_> {
                fn from(value: onERC721ReceivedReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for onERC721ReceivedReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for onERC721ReceivedCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Bytes,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = onERC721ReceivedReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<4>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "onERC721Received(address,address,uint256,bytes)";
            const SELECTOR: [u8; 4] = [21u8, 11u8, 122u8, 2u8];
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
                        &self._0,
                    ),
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self._1,
                    ),
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self._2,
                    ),
                    <alloy::sol_types::sol_data::Bytes as alloy_sol_types::SolType>::tokenize(
                        &self._3,
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `renounceRole(bytes32,address)` and selector `0x36568abe`.
    ```solidity
    function renounceRole(bytes32 role, address callerConfirmation) external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct renounceRoleCall {
        #[allow(missing_docs)]
        pub role: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub callerConfirmation: alloy::sol_types::private::Address,
    }
    ///Container type for the return parameters of the [`renounceRole(bytes32,address)`](renounceRoleCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct renounceRoleReturn {}
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
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::FixedBytes<32>,
                alloy::sol_types::private::Address,
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
            impl ::core::convert::From<renounceRoleCall> for UnderlyingRustTuple<'_> {
                fn from(value: renounceRoleCall) -> Self {
                    (value.role, value.callerConfirmation)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for renounceRoleCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        role: tuple.0,
                        callerConfirmation: tuple.1,
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<renounceRoleReturn> for UnderlyingRustTuple<'_> {
                fn from(value: renounceRoleReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for renounceRoleReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for renounceRoleCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = renounceRoleReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "renounceRole(bytes32,address)";
            const SELECTOR: [u8; 4] = [54u8, 86u8, 138u8, 190u8];
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
                    > as alloy_sol_types::SolType>::tokenize(&self.role),
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.callerConfirmation,
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `revokeRole(bytes32,address)` and selector `0xd547741f`.
    ```solidity
    function revokeRole(bytes32 role, address account) external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct revokeRoleCall {
        #[allow(missing_docs)]
        pub role: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub account: alloy::sol_types::private::Address,
    }
    ///Container type for the return parameters of the [`revokeRole(bytes32,address)`](revokeRoleCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct revokeRoleReturn {}
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
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::FixedBytes<32>,
                alloy::sol_types::private::Address,
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
            impl ::core::convert::From<revokeRoleCall> for UnderlyingRustTuple<'_> {
                fn from(value: revokeRoleCall) -> Self {
                    (value.role, value.account)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for revokeRoleCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        role: tuple.0,
                        account: tuple.1,
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<revokeRoleReturn> for UnderlyingRustTuple<'_> {
                fn from(value: revokeRoleReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for revokeRoleReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for revokeRoleCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = revokeRoleReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "revokeRole(bytes32,address)";
            const SELECTOR: [u8; 4] = [213u8, 71u8, 116u8, 31u8];
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
                    > as alloy_sol_types::SolType>::tokenize(&self.role),
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.account,
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `schedule(address,uint256,bytes,bytes32,bytes32,uint256)` and selector `0x01d5062a`.
    ```solidity
    function schedule(address target, uint256 value, bytes memory data, bytes32 predecessor, bytes32 salt, uint256 delay) external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct scheduleCall {
        #[allow(missing_docs)]
        pub target: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub value: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub data: alloy::sol_types::private::Bytes,
        #[allow(missing_docs)]
        pub predecessor: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub salt: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub delay: alloy::sol_types::private::primitives::aliases::U256,
    }
    ///Container type for the return parameters of the [`schedule(address,uint256,bytes,bytes32,bytes32,uint256)`](scheduleCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct scheduleReturn {}
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
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Bytes,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Uint<256>,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::Address,
                alloy::sol_types::private::primitives::aliases::U256,
                alloy::sol_types::private::Bytes,
                alloy::sol_types::private::FixedBytes<32>,
                alloy::sol_types::private::FixedBytes<32>,
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
            impl ::core::convert::From<scheduleCall> for UnderlyingRustTuple<'_> {
                fn from(value: scheduleCall) -> Self {
                    (
                        value.target,
                        value.value,
                        value.data,
                        value.predecessor,
                        value.salt,
                        value.delay,
                    )
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for scheduleCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        target: tuple.0,
                        value: tuple.1,
                        data: tuple.2,
                        predecessor: tuple.3,
                        salt: tuple.4,
                        delay: tuple.5,
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<scheduleReturn> for UnderlyingRustTuple<'_> {
                fn from(value: scheduleReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for scheduleReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for scheduleCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Bytes,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Uint<256>,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = scheduleReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str =
                "schedule(address,uint256,bytes,bytes32,bytes32,uint256)";
            const SELECTOR: [u8; 4] = [1u8, 213u8, 6u8, 42u8];
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
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::tokenize(&self.value),
                    <alloy::sol_types::sol_data::Bytes as alloy_sol_types::SolType>::tokenize(
                        &self.data,
                    ),
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.predecessor),
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.salt),
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::tokenize(&self.delay),
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `scheduleBatch(address[],uint256[],bytes[],bytes32,bytes32,uint256)` and selector `0x8f2a0bb0`.
    ```solidity
    function scheduleBatch(address[] memory targets, uint256[] memory values, bytes[] memory payloads, bytes32 predecessor, bytes32 salt, uint256 delay) external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct scheduleBatchCall {
        #[allow(missing_docs)]
        pub targets: alloy::sol_types::private::Vec<alloy::sol_types::private::Address>,
        #[allow(missing_docs)]
        pub values:
            alloy::sol_types::private::Vec<alloy::sol_types::private::primitives::aliases::U256>,
        #[allow(missing_docs)]
        pub payloads: alloy::sol_types::private::Vec<alloy::sol_types::private::Bytes>,
        #[allow(missing_docs)]
        pub predecessor: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub salt: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub delay: alloy::sol_types::private::primitives::aliases::U256,
    }
    ///Container type for the return parameters of the [`scheduleBatch(address[],uint256[],bytes[],bytes32,bytes32,uint256)`](scheduleBatchCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct scheduleBatchReturn {}
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
                alloy::sol_types::sol_data::Array<alloy::sol_types::sol_data::Address>,
                alloy::sol_types::sol_data::Array<alloy::sol_types::sol_data::Uint<256>>,
                alloy::sol_types::sol_data::Array<alloy::sol_types::sol_data::Bytes>,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Uint<256>,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::Vec<alloy::sol_types::private::Address>,
                alloy::sol_types::private::Vec<
                    alloy::sol_types::private::primitives::aliases::U256,
                >,
                alloy::sol_types::private::Vec<alloy::sol_types::private::Bytes>,
                alloy::sol_types::private::FixedBytes<32>,
                alloy::sol_types::private::FixedBytes<32>,
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
            impl ::core::convert::From<scheduleBatchCall> for UnderlyingRustTuple<'_> {
                fn from(value: scheduleBatchCall) -> Self {
                    (
                        value.targets,
                        value.values,
                        value.payloads,
                        value.predecessor,
                        value.salt,
                        value.delay,
                    )
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for scheduleBatchCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        targets: tuple.0,
                        values: tuple.1,
                        payloads: tuple.2,
                        predecessor: tuple.3,
                        salt: tuple.4,
                        delay: tuple.5,
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<scheduleBatchReturn> for UnderlyingRustTuple<'_> {
                fn from(value: scheduleBatchReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for scheduleBatchReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for scheduleBatchCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::Array<alloy::sol_types::sol_data::Address>,
                alloy::sol_types::sol_data::Array<alloy::sol_types::sol_data::Uint<256>>,
                alloy::sol_types::sol_data::Array<alloy::sol_types::sol_data::Bytes>,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Uint<256>,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = scheduleBatchReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str =
                "scheduleBatch(address[],uint256[],bytes[],bytes32,bytes32,uint256)";
            const SELECTOR: [u8; 4] = [143u8, 42u8, 11u8, 176u8];
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
                        alloy::sol_types::sol_data::Address,
                    > as alloy_sol_types::SolType>::tokenize(&self.targets),
                    <alloy::sol_types::sol_data::Array<
                        alloy::sol_types::sol_data::Uint<256>,
                    > as alloy_sol_types::SolType>::tokenize(&self.values),
                    <alloy::sol_types::sol_data::Array<
                        alloy::sol_types::sol_data::Bytes,
                    > as alloy_sol_types::SolType>::tokenize(&self.payloads),
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.predecessor),
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.salt),
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::tokenize(&self.delay),
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `supportsInterface(bytes4)` and selector `0x01ffc9a7`.
    ```solidity
    function supportsInterface(bytes4 interfaceId) external view returns (bool);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct supportsInterfaceCall {
        #[allow(missing_docs)]
        pub interfaceId: alloy::sol_types::private::FixedBytes<4>,
    }
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`supportsInterface(bytes4)`](supportsInterfaceCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct supportsInterfaceReturn {
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
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<4>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::FixedBytes<4>,);
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
            impl ::core::convert::From<supportsInterfaceCall> for UnderlyingRustTuple<'_> {
                fn from(value: supportsInterfaceCall) -> Self {
                    (value.interfaceId,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for supportsInterfaceCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        interfaceId: tuple.0,
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
            impl ::core::convert::From<supportsInterfaceReturn> for UnderlyingRustTuple<'_> {
                fn from(value: supportsInterfaceReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for supportsInterfaceReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for supportsInterfaceCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::FixedBytes<4>,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = supportsInterfaceReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "supportsInterface(bytes4)";
            const SELECTOR: [u8; 4] = [1u8, 255u8, 201u8, 167u8];
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
                        4,
                    > as alloy_sol_types::SolType>::tokenize(&self.interfaceId),
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `updateDelay(uint256)` and selector `0x64d62353`.
    ```solidity
    function updateDelay(uint256 newDelay) external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct updateDelayCall {
        #[allow(missing_docs)]
        pub newDelay: alloy::sol_types::private::primitives::aliases::U256,
    }
    ///Container type for the return parameters of the [`updateDelay(uint256)`](updateDelayCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct updateDelayReturn {}
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
            impl ::core::convert::From<updateDelayCall> for UnderlyingRustTuple<'_> {
                fn from(value: updateDelayCall) -> Self {
                    (value.newDelay,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for updateDelayCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { newDelay: tuple.0 }
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<updateDelayReturn> for UnderlyingRustTuple<'_> {
                fn from(value: updateDelayReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for updateDelayReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for updateDelayCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = updateDelayReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "updateDelay(uint256)";
            const SELECTOR: [u8; 4] = [100u8, 214u8, 35u8, 83u8];
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
                        &self.newDelay,
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
    ///Container for all the [`Timelock`](self) function calls.
    #[derive()]
    pub enum TimelockCalls {
        #[allow(missing_docs)]
        CANCELLER_ROLE(CANCELLER_ROLECall),
        #[allow(missing_docs)]
        DEFAULT_ADMIN_ROLE(DEFAULT_ADMIN_ROLECall),
        #[allow(missing_docs)]
        EXECUTOR_ROLE(EXECUTOR_ROLECall),
        #[allow(missing_docs)]
        PROPOSER_ROLE(PROPOSER_ROLECall),
        #[allow(missing_docs)]
        cancel(cancelCall),
        #[allow(missing_docs)]
        execute(executeCall),
        #[allow(missing_docs)]
        executeBatch(executeBatchCall),
        #[allow(missing_docs)]
        getMinDelay(getMinDelayCall),
        #[allow(missing_docs)]
        getOperationState(getOperationStateCall),
        #[allow(missing_docs)]
        getRoleAdmin(getRoleAdminCall),
        #[allow(missing_docs)]
        getTimestamp(getTimestampCall),
        #[allow(missing_docs)]
        grantRole(grantRoleCall),
        #[allow(missing_docs)]
        hasRole(hasRoleCall),
        #[allow(missing_docs)]
        hashOperation(hashOperationCall),
        #[allow(missing_docs)]
        hashOperationBatch(hashOperationBatchCall),
        #[allow(missing_docs)]
        isOperation(isOperationCall),
        #[allow(missing_docs)]
        isOperationDone(isOperationDoneCall),
        #[allow(missing_docs)]
        isOperationPending(isOperationPendingCall),
        #[allow(missing_docs)]
        isOperationReady(isOperationReadyCall),
        #[allow(missing_docs)]
        onERC1155BatchReceived(onERC1155BatchReceivedCall),
        #[allow(missing_docs)]
        onERC1155Received(onERC1155ReceivedCall),
        #[allow(missing_docs)]
        onERC721Received(onERC721ReceivedCall),
        #[allow(missing_docs)]
        renounceRole(renounceRoleCall),
        #[allow(missing_docs)]
        revokeRole(revokeRoleCall),
        #[allow(missing_docs)]
        schedule(scheduleCall),
        #[allow(missing_docs)]
        scheduleBatch(scheduleBatchCall),
        #[allow(missing_docs)]
        supportsInterface(supportsInterfaceCall),
        #[allow(missing_docs)]
        updateDelay(updateDelayCall),
    }
    #[automatically_derived]
    impl TimelockCalls {
        /// All the selectors of this enum.
        ///
        /// Note that the selectors might not be in the same order as the variants.
        /// No guarantees are made about the order of the selectors.
        ///
        /// Prefer using `SolInterface` methods instead.
        pub const SELECTORS: &'static [[u8; 4usize]] = &[
            [1u8, 213u8, 6u8, 42u8],
            [1u8, 255u8, 201u8, 167u8],
            [7u8, 189u8, 2u8, 101u8],
            [19u8, 64u8, 8u8, 211u8],
            [19u8, 188u8, 159u8, 32u8],
            [21u8, 11u8, 122u8, 2u8],
            [36u8, 138u8, 156u8, 163u8],
            [42u8, 176u8, 245u8, 41u8],
            [47u8, 47u8, 241u8, 93u8],
            [49u8, 213u8, 7u8, 80u8],
            [54u8, 86u8, 138u8, 190u8],
            [88u8, 75u8, 21u8, 62u8],
            [100u8, 214u8, 35u8, 83u8],
            [121u8, 88u8, 0u8, 76u8],
            [128u8, 101u8, 101u8, 127u8],
            [143u8, 42u8, 11u8, 176u8],
            [143u8, 97u8, 244u8, 245u8],
            [145u8, 209u8, 72u8, 84u8],
            [162u8, 23u8, 253u8, 223u8],
            [176u8, 142u8, 81u8, 192u8],
            [177u8, 197u8, 244u8, 39u8],
            [188u8, 25u8, 124u8, 129u8],
            [196u8, 210u8, 82u8, 245u8],
            [212u8, 92u8, 68u8, 53u8],
            [213u8, 71u8, 116u8, 31u8],
            [227u8, 131u8, 53u8, 229u8],
            [242u8, 58u8, 110u8, 97u8],
            [242u8, 122u8, 12u8, 146u8],
        ];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolInterface for TimelockCalls {
        const NAME: &'static str = "TimelockCalls";
        const MIN_DATA_LENGTH: usize = 0usize;
        const COUNT: usize = 28usize;
        #[inline]
        fn selector(&self) -> [u8; 4] {
            match self {
                Self::CANCELLER_ROLE(_) => {
                    <CANCELLER_ROLECall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::DEFAULT_ADMIN_ROLE(_) => {
                    <DEFAULT_ADMIN_ROLECall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::EXECUTOR_ROLE(_) => <EXECUTOR_ROLECall as alloy_sol_types::SolCall>::SELECTOR,
                Self::PROPOSER_ROLE(_) => <PROPOSER_ROLECall as alloy_sol_types::SolCall>::SELECTOR,
                Self::cancel(_) => <cancelCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::execute(_) => <executeCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::executeBatch(_) => <executeBatchCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::getMinDelay(_) => <getMinDelayCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::getOperationState(_) => {
                    <getOperationStateCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::getRoleAdmin(_) => <getRoleAdminCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::getTimestamp(_) => <getTimestampCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::grantRole(_) => <grantRoleCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::hasRole(_) => <hasRoleCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::hashOperation(_) => <hashOperationCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::hashOperationBatch(_) => {
                    <hashOperationBatchCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::isOperation(_) => <isOperationCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::isOperationDone(_) => {
                    <isOperationDoneCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::isOperationPending(_) => {
                    <isOperationPendingCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::isOperationReady(_) => {
                    <isOperationReadyCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::onERC1155BatchReceived(_) => {
                    <onERC1155BatchReceivedCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::onERC1155Received(_) => {
                    <onERC1155ReceivedCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::onERC721Received(_) => {
                    <onERC721ReceivedCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::renounceRole(_) => <renounceRoleCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::revokeRole(_) => <revokeRoleCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::schedule(_) => <scheduleCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::scheduleBatch(_) => <scheduleBatchCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::supportsInterface(_) => {
                    <supportsInterfaceCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::updateDelay(_) => <updateDelayCall as alloy_sol_types::SolCall>::SELECTOR,
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
            static DECODE_SHIMS: &[fn(&[u8], bool) -> alloy_sol_types::Result<TimelockCalls>] = &[
                {
                    fn schedule(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockCalls> {
                        <scheduleCall as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(TimelockCalls::schedule)
                    }
                    schedule
                },
                {
                    fn supportsInterface(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockCalls> {
                        <supportsInterfaceCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(TimelockCalls::supportsInterface)
                    }
                    supportsInterface
                },
                {
                    fn EXECUTOR_ROLE(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockCalls> {
                        <EXECUTOR_ROLECall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(TimelockCalls::EXECUTOR_ROLE)
                    }
                    EXECUTOR_ROLE
                },
                {
                    fn execute(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockCalls> {
                        <executeCall as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(TimelockCalls::execute)
                    }
                    execute
                },
                {
                    fn isOperationReady(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockCalls> {
                        <isOperationReadyCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(TimelockCalls::isOperationReady)
                    }
                    isOperationReady
                },
                {
                    fn onERC721Received(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockCalls> {
                        <onERC721ReceivedCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(TimelockCalls::onERC721Received)
                    }
                    onERC721Received
                },
                {
                    fn getRoleAdmin(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockCalls> {
                        <getRoleAdminCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(TimelockCalls::getRoleAdmin)
                    }
                    getRoleAdmin
                },
                {
                    fn isOperationDone(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockCalls> {
                        <isOperationDoneCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(TimelockCalls::isOperationDone)
                    }
                    isOperationDone
                },
                {
                    fn grantRole(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockCalls> {
                        <grantRoleCall as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(TimelockCalls::grantRole)
                    }
                    grantRole
                },
                {
                    fn isOperation(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockCalls> {
                        <isOperationCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(TimelockCalls::isOperation)
                    }
                    isOperation
                },
                {
                    fn renounceRole(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockCalls> {
                        <renounceRoleCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(TimelockCalls::renounceRole)
                    }
                    renounceRole
                },
                {
                    fn isOperationPending(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockCalls> {
                        <isOperationPendingCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(TimelockCalls::isOperationPending)
                    }
                    isOperationPending
                },
                {
                    fn updateDelay(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockCalls> {
                        <updateDelayCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(TimelockCalls::updateDelay)
                    }
                    updateDelay
                },
                {
                    fn getOperationState(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockCalls> {
                        <getOperationStateCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(TimelockCalls::getOperationState)
                    }
                    getOperationState
                },
                {
                    fn hashOperation(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockCalls> {
                        <hashOperationCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(TimelockCalls::hashOperation)
                    }
                    hashOperation
                },
                {
                    fn scheduleBatch(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockCalls> {
                        <scheduleBatchCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(TimelockCalls::scheduleBatch)
                    }
                    scheduleBatch
                },
                {
                    fn PROPOSER_ROLE(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockCalls> {
                        <PROPOSER_ROLECall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(TimelockCalls::PROPOSER_ROLE)
                    }
                    PROPOSER_ROLE
                },
                {
                    fn hasRole(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockCalls> {
                        <hasRoleCall as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(TimelockCalls::hasRole)
                    }
                    hasRole
                },
                {
                    fn DEFAULT_ADMIN_ROLE(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockCalls> {
                        <DEFAULT_ADMIN_ROLECall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(TimelockCalls::DEFAULT_ADMIN_ROLE)
                    }
                    DEFAULT_ADMIN_ROLE
                },
                {
                    fn CANCELLER_ROLE(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockCalls> {
                        <CANCELLER_ROLECall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(TimelockCalls::CANCELLER_ROLE)
                    }
                    CANCELLER_ROLE
                },
                {
                    fn hashOperationBatch(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockCalls> {
                        <hashOperationBatchCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(TimelockCalls::hashOperationBatch)
                    }
                    hashOperationBatch
                },
                {
                    fn onERC1155BatchReceived(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockCalls> {
                        <onERC1155BatchReceivedCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(TimelockCalls::onERC1155BatchReceived)
                    }
                    onERC1155BatchReceived
                },
                {
                    fn cancel(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockCalls> {
                        <cancelCall as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(TimelockCalls::cancel)
                    }
                    cancel
                },
                {
                    fn getTimestamp(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockCalls> {
                        <getTimestampCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(TimelockCalls::getTimestamp)
                    }
                    getTimestamp
                },
                {
                    fn revokeRole(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockCalls> {
                        <revokeRoleCall as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(TimelockCalls::revokeRole)
                    }
                    revokeRole
                },
                {
                    fn executeBatch(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockCalls> {
                        <executeBatchCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(TimelockCalls::executeBatch)
                    }
                    executeBatch
                },
                {
                    fn onERC1155Received(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockCalls> {
                        <onERC1155ReceivedCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(TimelockCalls::onERC1155Received)
                    }
                    onERC1155Received
                },
                {
                    fn getMinDelay(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockCalls> {
                        <getMinDelayCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(TimelockCalls::getMinDelay)
                    }
                    getMinDelay
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
                Self::CANCELLER_ROLE(inner) => {
                    <CANCELLER_ROLECall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::DEFAULT_ADMIN_ROLE(inner) => {
                    <DEFAULT_ADMIN_ROLECall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::EXECUTOR_ROLE(inner) => {
                    <EXECUTOR_ROLECall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::PROPOSER_ROLE(inner) => {
                    <PROPOSER_ROLECall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::cancel(inner) => {
                    <cancelCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::execute(inner) => {
                    <executeCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::executeBatch(inner) => {
                    <executeBatchCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::getMinDelay(inner) => {
                    <getMinDelayCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::getOperationState(inner) => {
                    <getOperationStateCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::getRoleAdmin(inner) => {
                    <getRoleAdminCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::getTimestamp(inner) => {
                    <getTimestampCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::grantRole(inner) => {
                    <grantRoleCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::hasRole(inner) => {
                    <hasRoleCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::hashOperation(inner) => {
                    <hashOperationCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::hashOperationBatch(inner) => {
                    <hashOperationBatchCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::isOperation(inner) => {
                    <isOperationCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::isOperationDone(inner) => {
                    <isOperationDoneCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::isOperationPending(inner) => {
                    <isOperationPendingCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::isOperationReady(inner) => {
                    <isOperationReadyCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::onERC1155BatchReceived(inner) => {
                    <onERC1155BatchReceivedCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                },
                Self::onERC1155Received(inner) => {
                    <onERC1155ReceivedCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::onERC721Received(inner) => {
                    <onERC721ReceivedCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::renounceRole(inner) => {
                    <renounceRoleCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::revokeRole(inner) => {
                    <revokeRoleCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::schedule(inner) => {
                    <scheduleCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::scheduleBatch(inner) => {
                    <scheduleBatchCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::supportsInterface(inner) => {
                    <supportsInterfaceCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::updateDelay(inner) => {
                    <updateDelayCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
            }
        }
        #[inline]
        fn abi_encode_raw(&self, out: &mut alloy_sol_types::private::Vec<u8>) {
            match self {
                Self::CANCELLER_ROLE(inner) => {
                    <CANCELLER_ROLECall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::DEFAULT_ADMIN_ROLE(inner) => {
                    <DEFAULT_ADMIN_ROLECall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::EXECUTOR_ROLE(inner) => {
                    <EXECUTOR_ROLECall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::PROPOSER_ROLE(inner) => {
                    <PROPOSER_ROLECall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::cancel(inner) => {
                    <cancelCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::execute(inner) => {
                    <executeCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::executeBatch(inner) => {
                    <executeBatchCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::getMinDelay(inner) => {
                    <getMinDelayCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::getOperationState(inner) => {
                    <getOperationStateCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::getRoleAdmin(inner) => {
                    <getRoleAdminCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::getTimestamp(inner) => {
                    <getTimestampCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::grantRole(inner) => {
                    <grantRoleCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::hasRole(inner) => {
                    <hasRoleCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::hashOperation(inner) => {
                    <hashOperationCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::hashOperationBatch(inner) => {
                    <hashOperationBatchCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::isOperation(inner) => {
                    <isOperationCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::isOperationDone(inner) => {
                    <isOperationDoneCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::isOperationPending(inner) => {
                    <isOperationPendingCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::isOperationReady(inner) => {
                    <isOperationReadyCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::onERC1155BatchReceived(inner) => {
                    <onERC1155BatchReceivedCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::onERC1155Received(inner) => {
                    <onERC1155ReceivedCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::onERC721Received(inner) => {
                    <onERC721ReceivedCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::renounceRole(inner) => {
                    <renounceRoleCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::revokeRole(inner) => {
                    <revokeRoleCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::schedule(inner) => {
                    <scheduleCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::scheduleBatch(inner) => {
                    <scheduleBatchCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::supportsInterface(inner) => {
                    <supportsInterfaceCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::updateDelay(inner) => {
                    <updateDelayCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
            }
        }
    }
    ///Container for all the [`Timelock`](self) custom errors.
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub enum TimelockErrors {
        #[allow(missing_docs)]
        AccessControlBadConfirmation(AccessControlBadConfirmation),
        #[allow(missing_docs)]
        AccessControlUnauthorizedAccount(AccessControlUnauthorizedAccount),
        #[allow(missing_docs)]
        FailedInnerCall(FailedInnerCall),
        #[allow(missing_docs)]
        TimelockInsufficientDelay(TimelockInsufficientDelay),
        #[allow(missing_docs)]
        TimelockInvalidOperationLength(TimelockInvalidOperationLength),
        #[allow(missing_docs)]
        TimelockUnauthorizedCaller(TimelockUnauthorizedCaller),
        #[allow(missing_docs)]
        TimelockUnexecutedPredecessor(TimelockUnexecutedPredecessor),
        #[allow(missing_docs)]
        TimelockUnexpectedOperationState(TimelockUnexpectedOperationState),
    }
    #[automatically_derived]
    impl TimelockErrors {
        /// All the selectors of this enum.
        ///
        /// Note that the selectors might not be in the same order as the variants.
        /// No guarantees are made about the order of the selectors.
        ///
        /// Prefer using `SolInterface` methods instead.
        pub const SELECTORS: &'static [[u8; 4usize]] = &[
            [20u8, 37u8, 234u8, 66u8],
            [84u8, 51u8, 102u8, 9u8],
            [94u8, 173u8, 142u8, 181u8],
            [102u8, 151u8, 178u8, 50u8],
            [144u8, 169u8, 166u8, 24u8],
            [226u8, 81u8, 125u8, 63u8],
            [226u8, 133u8, 12u8, 89u8],
            [255u8, 176u8, 50u8, 17u8],
        ];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolInterface for TimelockErrors {
        const NAME: &'static str = "TimelockErrors";
        const MIN_DATA_LENGTH: usize = 0usize;
        const COUNT: usize = 8usize;
        #[inline]
        fn selector(&self) -> [u8; 4] {
            match self {
                Self::AccessControlBadConfirmation(_) => {
                    <AccessControlBadConfirmation as alloy_sol_types::SolError>::SELECTOR
                },
                Self::AccessControlUnauthorizedAccount(_) => {
                    <AccessControlUnauthorizedAccount as alloy_sol_types::SolError>::SELECTOR
                },
                Self::FailedInnerCall(_) => {
                    <FailedInnerCall as alloy_sol_types::SolError>::SELECTOR
                },
                Self::TimelockInsufficientDelay(_) => {
                    <TimelockInsufficientDelay as alloy_sol_types::SolError>::SELECTOR
                },
                Self::TimelockInvalidOperationLength(_) => {
                    <TimelockInvalidOperationLength as alloy_sol_types::SolError>::SELECTOR
                },
                Self::TimelockUnauthorizedCaller(_) => {
                    <TimelockUnauthorizedCaller as alloy_sol_types::SolError>::SELECTOR
                },
                Self::TimelockUnexecutedPredecessor(_) => {
                    <TimelockUnexecutedPredecessor as alloy_sol_types::SolError>::SELECTOR
                },
                Self::TimelockUnexpectedOperationState(_) => {
                    <TimelockUnexpectedOperationState as alloy_sol_types::SolError>::SELECTOR
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
            static DECODE_SHIMS: &[fn(&[u8], bool) -> alloy_sol_types::Result<TimelockErrors>] = &[
                {
                    fn FailedInnerCall(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockErrors> {
                        <FailedInnerCall as alloy_sol_types::SolError>::abi_decode_raw(
                            data, validate,
                        )
                        .map(TimelockErrors::FailedInnerCall)
                    }
                    FailedInnerCall
                },
                {
                    fn TimelockInsufficientDelay(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockErrors> {
                        <TimelockInsufficientDelay as alloy_sol_types::SolError>::abi_decode_raw(
                            data, validate,
                        )
                        .map(TimelockErrors::TimelockInsufficientDelay)
                    }
                    TimelockInsufficientDelay
                },
                {
                    fn TimelockUnexpectedOperationState(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockErrors> {
                        <TimelockUnexpectedOperationState as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(TimelockErrors::TimelockUnexpectedOperationState)
                    }
                    TimelockUnexpectedOperationState
                },
                {
                    fn AccessControlBadConfirmation(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockErrors> {
                        <AccessControlBadConfirmation as alloy_sol_types::SolError>::abi_decode_raw(
                            data, validate,
                        )
                        .map(TimelockErrors::AccessControlBadConfirmation)
                    }
                    AccessControlBadConfirmation
                },
                {
                    fn TimelockUnexecutedPredecessor(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockErrors> {
                        <TimelockUnexecutedPredecessor as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(TimelockErrors::TimelockUnexecutedPredecessor)
                    }
                    TimelockUnexecutedPredecessor
                },
                {
                    fn AccessControlUnauthorizedAccount(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockErrors> {
                        <AccessControlUnauthorizedAccount as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(TimelockErrors::AccessControlUnauthorizedAccount)
                    }
                    AccessControlUnauthorizedAccount
                },
                {
                    fn TimelockUnauthorizedCaller(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockErrors> {
                        <TimelockUnauthorizedCaller as alloy_sol_types::SolError>::abi_decode_raw(
                            data, validate,
                        )
                        .map(TimelockErrors::TimelockUnauthorizedCaller)
                    }
                    TimelockUnauthorizedCaller
                },
                {
                    fn TimelockInvalidOperationLength(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<TimelockErrors> {
                        <TimelockInvalidOperationLength as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(TimelockErrors::TimelockInvalidOperationLength)
                    }
                    TimelockInvalidOperationLength
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
                Self::AccessControlBadConfirmation(inner) => {
                    <AccessControlBadConfirmation as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::AccessControlUnauthorizedAccount(inner) => {
                    <AccessControlUnauthorizedAccount as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::FailedInnerCall(inner) => {
                    <FailedInnerCall as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::TimelockInsufficientDelay(inner) => {
                    <TimelockInsufficientDelay as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::TimelockInvalidOperationLength(inner) => {
                    <TimelockInvalidOperationLength as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::TimelockUnauthorizedCaller(inner) => {
                    <TimelockUnauthorizedCaller as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::TimelockUnexecutedPredecessor(inner) => {
                    <TimelockUnexecutedPredecessor as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::TimelockUnexpectedOperationState(inner) => {
                    <TimelockUnexpectedOperationState as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
            }
        }
        #[inline]
        fn abi_encode_raw(&self, out: &mut alloy_sol_types::private::Vec<u8>) {
            match self {
                Self::AccessControlBadConfirmation(inner) => {
                    <AccessControlBadConfirmation as alloy_sol_types::SolError>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::AccessControlUnauthorizedAccount(inner) => {
                    <AccessControlUnauthorizedAccount as alloy_sol_types::SolError>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::FailedInnerCall(inner) => {
                    <FailedInnerCall as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                },
                Self::TimelockInsufficientDelay(inner) => {
                    <TimelockInsufficientDelay as alloy_sol_types::SolError>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::TimelockInvalidOperationLength(inner) => {
                    <TimelockInvalidOperationLength as alloy_sol_types::SolError>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::TimelockUnauthorizedCaller(inner) => {
                    <TimelockUnauthorizedCaller as alloy_sol_types::SolError>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::TimelockUnexecutedPredecessor(inner) => {
                    <TimelockUnexecutedPredecessor as alloy_sol_types::SolError>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::TimelockUnexpectedOperationState(inner) => {
                    <TimelockUnexpectedOperationState as alloy_sol_types::SolError>::abi_encode_raw(
                        inner, out,
                    )
                },
            }
        }
    }
    ///Container for all the [`Timelock`](self) events.
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub enum TimelockEvents {
        #[allow(missing_docs)]
        CallExecuted(CallExecuted),
        #[allow(missing_docs)]
        CallSalt(CallSalt),
        #[allow(missing_docs)]
        CallScheduled(CallScheduled),
        #[allow(missing_docs)]
        Cancelled(Cancelled),
        #[allow(missing_docs)]
        MinDelayChange(MinDelayChange),
        #[allow(missing_docs)]
        RoleAdminChanged(RoleAdminChanged),
        #[allow(missing_docs)]
        RoleGranted(RoleGranted),
        #[allow(missing_docs)]
        RoleRevoked(RoleRevoked),
    }
    #[automatically_derived]
    impl TimelockEvents {
        /// All the selectors of this enum.
        ///
        /// Note that the selectors might not be in the same order as the variants.
        /// No guarantees are made about the order of the selectors.
        ///
        /// Prefer using `SolInterface` methods instead.
        pub const SELECTORS: &'static [[u8; 32usize]] = &[
            [
                17u8, 194u8, 79u8, 78u8, 173u8, 22u8, 80u8, 124u8, 105u8, 172u8, 70u8, 127u8,
                189u8, 94u8, 78u8, 237u8, 95u8, 181u8, 198u8, 153u8, 98u8, 109u8, 44u8, 198u8,
                214u8, 100u8, 33u8, 223u8, 37u8, 56u8, 134u8, 213u8,
            ],
            [
                32u8, 253u8, 165u8, 253u8, 39u8, 161u8, 234u8, 123u8, 245u8, 185u8, 86u8, 127u8,
                20u8, 58u8, 197u8, 71u8, 11u8, 176u8, 89u8, 55u8, 74u8, 39u8, 232u8, 246u8, 124u8,
                180u8, 79u8, 148u8, 111u8, 109u8, 3u8, 135u8,
            ],
            [
                47u8, 135u8, 136u8, 17u8, 126u8, 126u8, 255u8, 29u8, 130u8, 233u8, 38u8, 236u8,
                121u8, 73u8, 1u8, 209u8, 124u8, 120u8, 2u8, 74u8, 80u8, 39u8, 9u8, 64u8, 48u8,
                69u8, 64u8, 167u8, 51u8, 101u8, 111u8, 13u8,
            ],
            [
                76u8, 244u8, 65u8, 12u8, 197u8, 112u8, 64u8, 228u8, 72u8, 98u8, 239u8, 15u8, 69u8,
                243u8, 221u8, 90u8, 94u8, 2u8, 219u8, 142u8, 184u8, 173u8, 214u8, 72u8, 212u8,
                176u8, 226u8, 54u8, 241u8, 208u8, 125u8, 202u8,
            ],
            [
                186u8, 161u8, 235u8, 34u8, 242u8, 164u8, 146u8, 186u8, 26u8, 95u8, 234u8, 97u8,
                184u8, 223u8, 77u8, 39u8, 198u8, 200u8, 181u8, 243u8, 151u8, 30u8, 99u8, 187u8,
                88u8, 250u8, 20u8, 255u8, 114u8, 238u8, 219u8, 112u8,
            ],
            [
                189u8, 121u8, 184u8, 111u8, 254u8, 10u8, 184u8, 232u8, 119u8, 97u8, 81u8, 81u8,
                66u8, 23u8, 205u8, 124u8, 172u8, 213u8, 44u8, 144u8, 159u8, 102u8, 71u8, 92u8,
                58u8, 244u8, 78u8, 18u8, 159u8, 11u8, 0u8, 255u8,
            ],
            [
                194u8, 97u8, 126u8, 250u8, 105u8, 186u8, 182u8, 103u8, 130u8, 250u8, 33u8, 149u8,
                67u8, 113u8, 67u8, 56u8, 72u8, 156u8, 78u8, 158u8, 23u8, 130u8, 113u8, 86u8, 10u8,
                145u8, 184u8, 44u8, 63u8, 97u8, 43u8, 88u8,
            ],
            [
                246u8, 57u8, 31u8, 92u8, 50u8, 217u8, 198u8, 157u8, 42u8, 71u8, 234u8, 103u8, 11u8,
                68u8, 41u8, 116u8, 181u8, 57u8, 53u8, 209u8, 237u8, 199u8, 253u8, 100u8, 235u8,
                33u8, 224u8, 71u8, 168u8, 57u8, 23u8, 27u8,
            ],
        ];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolEventInterface for TimelockEvents {
        const NAME: &'static str = "TimelockEvents";
        const COUNT: usize = 8usize;
        fn decode_raw_log(
            topics: &[alloy_sol_types::Word],
            data: &[u8],
            validate: bool,
        ) -> alloy_sol_types::Result<Self> {
            match topics.first().copied() {
                Some(<CallExecuted as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <CallExecuted as alloy_sol_types::SolEvent>::decode_raw_log(
                        topics, data, validate,
                    )
                    .map(Self::CallExecuted)
                },
                Some(<CallSalt as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <CallSalt as alloy_sol_types::SolEvent>::decode_raw_log(topics, data, validate)
                        .map(Self::CallSalt)
                },
                Some(<CallScheduled as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <CallScheduled as alloy_sol_types::SolEvent>::decode_raw_log(
                        topics, data, validate,
                    )
                    .map(Self::CallScheduled)
                },
                Some(<Cancelled as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <Cancelled as alloy_sol_types::SolEvent>::decode_raw_log(topics, data, validate)
                        .map(Self::Cancelled)
                },
                Some(<MinDelayChange as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <MinDelayChange as alloy_sol_types::SolEvent>::decode_raw_log(
                        topics, data, validate,
                    )
                    .map(Self::MinDelayChange)
                },
                Some(<RoleAdminChanged as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <RoleAdminChanged as alloy_sol_types::SolEvent>::decode_raw_log(
                        topics, data, validate,
                    )
                    .map(Self::RoleAdminChanged)
                },
                Some(<RoleGranted as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <RoleGranted as alloy_sol_types::SolEvent>::decode_raw_log(
                        topics, data, validate,
                    )
                    .map(Self::RoleGranted)
                },
                Some(<RoleRevoked as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <RoleRevoked as alloy_sol_types::SolEvent>::decode_raw_log(
                        topics, data, validate,
                    )
                    .map(Self::RoleRevoked)
                },
                _ => alloy_sol_types::private::Err(alloy_sol_types::Error::InvalidLog {
                    name: <Self as alloy_sol_types::SolEventInterface>::NAME,
                    log: alloy_sol_types::private::Box::new(
                        alloy_sol_types::private::LogData::new_unchecked(
                            topics.to_vec(),
                            data.to_vec().into(),
                        ),
                    ),
                }),
            }
        }
    }
    #[automatically_derived]
    impl alloy_sol_types::private::IntoLogData for TimelockEvents {
        fn to_log_data(&self) -> alloy_sol_types::private::LogData {
            match self {
                Self::CallExecuted(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                },
                Self::CallSalt(inner) => alloy_sol_types::private::IntoLogData::to_log_data(inner),
                Self::CallScheduled(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                },
                Self::Cancelled(inner) => alloy_sol_types::private::IntoLogData::to_log_data(inner),
                Self::MinDelayChange(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                },
                Self::RoleAdminChanged(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                },
                Self::RoleGranted(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                },
                Self::RoleRevoked(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                },
            }
        }
        fn into_log_data(self) -> alloy_sol_types::private::LogData {
            match self {
                Self::CallExecuted(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                },
                Self::CallSalt(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                },
                Self::CallScheduled(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                },
                Self::Cancelled(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                },
                Self::MinDelayChange(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                },
                Self::RoleAdminChanged(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                },
                Self::RoleGranted(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                },
                Self::RoleRevoked(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                },
            }
        }
    }
    use alloy::contract as alloy_contract;
    /**Creates a new wrapper around an on-chain [`Timelock`](self) contract instance.

    See the [wrapper's documentation](`TimelockInstance`) for more details.*/
    #[inline]
    pub const fn new<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    >(
        address: alloy_sol_types::private::Address,
        provider: P,
    ) -> TimelockInstance<T, P, N> {
        TimelockInstance::<T, P, N>::new(address, provider)
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
        minDelay: alloy::sol_types::private::primitives::aliases::U256,
        proposers: alloy::sol_types::private::Vec<alloy::sol_types::private::Address>,
        executors: alloy::sol_types::private::Vec<alloy::sol_types::private::Address>,
        admin: alloy::sol_types::private::Address,
    ) -> impl ::core::future::Future<Output = alloy_contract::Result<TimelockInstance<T, P, N>>>
    {
        TimelockInstance::<T, P, N>::deploy(provider, minDelay, proposers, executors, admin)
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
        minDelay: alloy::sol_types::private::primitives::aliases::U256,
        proposers: alloy::sol_types::private::Vec<alloy::sol_types::private::Address>,
        executors: alloy::sol_types::private::Vec<alloy::sol_types::private::Address>,
        admin: alloy::sol_types::private::Address,
    ) -> alloy_contract::RawCallBuilder<T, P, N> {
        TimelockInstance::<T, P, N>::deploy_builder(provider, minDelay, proposers, executors, admin)
    }
    /**A [`Timelock`](self) instance.

    Contains type-safe methods for interacting with an on-chain instance of the
    [`Timelock`](self) contract located at a given `address`, using a given
    provider `P`.

    If the contract bytecode is available (see the [`sol!`](alloy_sol_types::sol!)
    documentation on how to provide it), the `deploy` and `deploy_builder` methods can
    be used to deploy a new instance of the contract.

    See the [module-level documentation](self) for all the available methods.*/
    #[derive(Clone)]
    pub struct TimelockInstance<T, P, N = alloy_contract::private::Ethereum> {
        address: alloy_sol_types::private::Address,
        provider: P,
        _network_transport: ::core::marker::PhantomData<(N, T)>,
    }
    #[automatically_derived]
    impl<T, P, N> ::core::fmt::Debug for TimelockInstance<T, P, N> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple("TimelockInstance")
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
        > TimelockInstance<T, P, N>
    {
        /**Creates a new wrapper around an on-chain [`Timelock`](self) contract instance.

        See the [wrapper's documentation](`TimelockInstance`) for more details.*/
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
            minDelay: alloy::sol_types::private::primitives::aliases::U256,
            proposers: alloy::sol_types::private::Vec<alloy::sol_types::private::Address>,
            executors: alloy::sol_types::private::Vec<alloy::sol_types::private::Address>,
            admin: alloy::sol_types::private::Address,
        ) -> alloy_contract::Result<TimelockInstance<T, P, N>> {
            let call_builder =
                Self::deploy_builder(provider, minDelay, proposers, executors, admin);
            let contract_address = call_builder.deploy().await?;
            Ok(Self::new(contract_address, call_builder.provider))
        }
        /**Creates a `RawCallBuilder` for deploying this contract using the given `provider`
        and constructor arguments, if any.

        This is a simple wrapper around creating a `RawCallBuilder` with the data set to
        the bytecode concatenated with the constructor's ABI-encoded arguments.*/
        #[inline]
        pub fn deploy_builder(
            provider: P,
            minDelay: alloy::sol_types::private::primitives::aliases::U256,
            proposers: alloy::sol_types::private::Vec<alloy::sol_types::private::Address>,
            executors: alloy::sol_types::private::Vec<alloy::sol_types::private::Address>,
            admin: alloy::sol_types::private::Address,
        ) -> alloy_contract::RawCallBuilder<T, P, N> {
            alloy_contract::RawCallBuilder::new_raw_deploy(
                provider,
                [
                    &BYTECODE[..],
                    &alloy_sol_types::SolConstructor::abi_encode(&constructorCall {
                        minDelay,
                        proposers,
                        executors,
                        admin,
                    })[..],
                ]
                .concat()
                .into(),
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
    impl<T, P: ::core::clone::Clone, N> TimelockInstance<T, &P, N> {
        /// Clones the provider and returns a new instance with the cloned provider.
        #[inline]
        pub fn with_cloned_provider(self) -> TimelockInstance<T, P, N> {
            TimelockInstance {
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
        > TimelockInstance<T, P, N>
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
        ///Creates a new call builder for the [`CANCELLER_ROLE`] function.
        pub fn CANCELLER_ROLE(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, CANCELLER_ROLECall, N> {
            self.call_builder(&CANCELLER_ROLECall {})
        }
        ///Creates a new call builder for the [`DEFAULT_ADMIN_ROLE`] function.
        pub fn DEFAULT_ADMIN_ROLE(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, DEFAULT_ADMIN_ROLECall, N> {
            self.call_builder(&DEFAULT_ADMIN_ROLECall {})
        }
        ///Creates a new call builder for the [`EXECUTOR_ROLE`] function.
        pub fn EXECUTOR_ROLE(&self) -> alloy_contract::SolCallBuilder<T, &P, EXECUTOR_ROLECall, N> {
            self.call_builder(&EXECUTOR_ROLECall {})
        }
        ///Creates a new call builder for the [`PROPOSER_ROLE`] function.
        pub fn PROPOSER_ROLE(&self) -> alloy_contract::SolCallBuilder<T, &P, PROPOSER_ROLECall, N> {
            self.call_builder(&PROPOSER_ROLECall {})
        }
        ///Creates a new call builder for the [`cancel`] function.
        pub fn cancel(
            &self,
            id: alloy::sol_types::private::FixedBytes<32>,
        ) -> alloy_contract::SolCallBuilder<T, &P, cancelCall, N> {
            self.call_builder(&cancelCall { id })
        }
        ///Creates a new call builder for the [`execute`] function.
        pub fn execute(
            &self,
            target: alloy::sol_types::private::Address,
            value: alloy::sol_types::private::primitives::aliases::U256,
            payload: alloy::sol_types::private::Bytes,
            predecessor: alloy::sol_types::private::FixedBytes<32>,
            salt: alloy::sol_types::private::FixedBytes<32>,
        ) -> alloy_contract::SolCallBuilder<T, &P, executeCall, N> {
            self.call_builder(&executeCall {
                target,
                value,
                payload,
                predecessor,
                salt,
            })
        }
        ///Creates a new call builder for the [`executeBatch`] function.
        pub fn executeBatch(
            &self,
            targets: alloy::sol_types::private::Vec<alloy::sol_types::private::Address>,
            values: alloy::sol_types::private::Vec<
                alloy::sol_types::private::primitives::aliases::U256,
            >,
            payloads: alloy::sol_types::private::Vec<alloy::sol_types::private::Bytes>,
            predecessor: alloy::sol_types::private::FixedBytes<32>,
            salt: alloy::sol_types::private::FixedBytes<32>,
        ) -> alloy_contract::SolCallBuilder<T, &P, executeBatchCall, N> {
            self.call_builder(&executeBatchCall {
                targets,
                values,
                payloads,
                predecessor,
                salt,
            })
        }
        ///Creates a new call builder for the [`getMinDelay`] function.
        pub fn getMinDelay(&self) -> alloy_contract::SolCallBuilder<T, &P, getMinDelayCall, N> {
            self.call_builder(&getMinDelayCall {})
        }
        ///Creates a new call builder for the [`getOperationState`] function.
        pub fn getOperationState(
            &self,
            id: alloy::sol_types::private::FixedBytes<32>,
        ) -> alloy_contract::SolCallBuilder<T, &P, getOperationStateCall, N> {
            self.call_builder(&getOperationStateCall { id })
        }
        ///Creates a new call builder for the [`getRoleAdmin`] function.
        pub fn getRoleAdmin(
            &self,
            role: alloy::sol_types::private::FixedBytes<32>,
        ) -> alloy_contract::SolCallBuilder<T, &P, getRoleAdminCall, N> {
            self.call_builder(&getRoleAdminCall { role })
        }
        ///Creates a new call builder for the [`getTimestamp`] function.
        pub fn getTimestamp(
            &self,
            id: alloy::sol_types::private::FixedBytes<32>,
        ) -> alloy_contract::SolCallBuilder<T, &P, getTimestampCall, N> {
            self.call_builder(&getTimestampCall { id })
        }
        ///Creates a new call builder for the [`grantRole`] function.
        pub fn grantRole(
            &self,
            role: alloy::sol_types::private::FixedBytes<32>,
            account: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, grantRoleCall, N> {
            self.call_builder(&grantRoleCall { role, account })
        }
        ///Creates a new call builder for the [`hasRole`] function.
        pub fn hasRole(
            &self,
            role: alloy::sol_types::private::FixedBytes<32>,
            account: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, hasRoleCall, N> {
            self.call_builder(&hasRoleCall { role, account })
        }
        ///Creates a new call builder for the [`hashOperation`] function.
        pub fn hashOperation(
            &self,
            target: alloy::sol_types::private::Address,
            value: alloy::sol_types::private::primitives::aliases::U256,
            data: alloy::sol_types::private::Bytes,
            predecessor: alloy::sol_types::private::FixedBytes<32>,
            salt: alloy::sol_types::private::FixedBytes<32>,
        ) -> alloy_contract::SolCallBuilder<T, &P, hashOperationCall, N> {
            self.call_builder(&hashOperationCall {
                target,
                value,
                data,
                predecessor,
                salt,
            })
        }
        ///Creates a new call builder for the [`hashOperationBatch`] function.
        pub fn hashOperationBatch(
            &self,
            targets: alloy::sol_types::private::Vec<alloy::sol_types::private::Address>,
            values: alloy::sol_types::private::Vec<
                alloy::sol_types::private::primitives::aliases::U256,
            >,
            payloads: alloy::sol_types::private::Vec<alloy::sol_types::private::Bytes>,
            predecessor: alloy::sol_types::private::FixedBytes<32>,
            salt: alloy::sol_types::private::FixedBytes<32>,
        ) -> alloy_contract::SolCallBuilder<T, &P, hashOperationBatchCall, N> {
            self.call_builder(&hashOperationBatchCall {
                targets,
                values,
                payloads,
                predecessor,
                salt,
            })
        }
        ///Creates a new call builder for the [`isOperation`] function.
        pub fn isOperation(
            &self,
            id: alloy::sol_types::private::FixedBytes<32>,
        ) -> alloy_contract::SolCallBuilder<T, &P, isOperationCall, N> {
            self.call_builder(&isOperationCall { id })
        }
        ///Creates a new call builder for the [`isOperationDone`] function.
        pub fn isOperationDone(
            &self,
            id: alloy::sol_types::private::FixedBytes<32>,
        ) -> alloy_contract::SolCallBuilder<T, &P, isOperationDoneCall, N> {
            self.call_builder(&isOperationDoneCall { id })
        }
        ///Creates a new call builder for the [`isOperationPending`] function.
        pub fn isOperationPending(
            &self,
            id: alloy::sol_types::private::FixedBytes<32>,
        ) -> alloy_contract::SolCallBuilder<T, &P, isOperationPendingCall, N> {
            self.call_builder(&isOperationPendingCall { id })
        }
        ///Creates a new call builder for the [`isOperationReady`] function.
        pub fn isOperationReady(
            &self,
            id: alloy::sol_types::private::FixedBytes<32>,
        ) -> alloy_contract::SolCallBuilder<T, &P, isOperationReadyCall, N> {
            self.call_builder(&isOperationReadyCall { id })
        }
        ///Creates a new call builder for the [`onERC1155BatchReceived`] function.
        pub fn onERC1155BatchReceived(
            &self,
            _0: alloy::sol_types::private::Address,
            _1: alloy::sol_types::private::Address,
            _2: alloy::sol_types::private::Vec<
                alloy::sol_types::private::primitives::aliases::U256,
            >,
            _3: alloy::sol_types::private::Vec<
                alloy::sol_types::private::primitives::aliases::U256,
            >,
            _4: alloy::sol_types::private::Bytes,
        ) -> alloy_contract::SolCallBuilder<T, &P, onERC1155BatchReceivedCall, N> {
            self.call_builder(&onERC1155BatchReceivedCall { _0, _1, _2, _3, _4 })
        }
        ///Creates a new call builder for the [`onERC1155Received`] function.
        pub fn onERC1155Received(
            &self,
            _0: alloy::sol_types::private::Address,
            _1: alloy::sol_types::private::Address,
            _2: alloy::sol_types::private::primitives::aliases::U256,
            _3: alloy::sol_types::private::primitives::aliases::U256,
            _4: alloy::sol_types::private::Bytes,
        ) -> alloy_contract::SolCallBuilder<T, &P, onERC1155ReceivedCall, N> {
            self.call_builder(&onERC1155ReceivedCall { _0, _1, _2, _3, _4 })
        }
        ///Creates a new call builder for the [`onERC721Received`] function.
        pub fn onERC721Received(
            &self,
            _0: alloy::sol_types::private::Address,
            _1: alloy::sol_types::private::Address,
            _2: alloy::sol_types::private::primitives::aliases::U256,
            _3: alloy::sol_types::private::Bytes,
        ) -> alloy_contract::SolCallBuilder<T, &P, onERC721ReceivedCall, N> {
            self.call_builder(&onERC721ReceivedCall { _0, _1, _2, _3 })
        }
        ///Creates a new call builder for the [`renounceRole`] function.
        pub fn renounceRole(
            &self,
            role: alloy::sol_types::private::FixedBytes<32>,
            callerConfirmation: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, renounceRoleCall, N> {
            self.call_builder(&renounceRoleCall {
                role,
                callerConfirmation,
            })
        }
        ///Creates a new call builder for the [`revokeRole`] function.
        pub fn revokeRole(
            &self,
            role: alloy::sol_types::private::FixedBytes<32>,
            account: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, revokeRoleCall, N> {
            self.call_builder(&revokeRoleCall { role, account })
        }
        ///Creates a new call builder for the [`schedule`] function.
        pub fn schedule(
            &self,
            target: alloy::sol_types::private::Address,
            value: alloy::sol_types::private::primitives::aliases::U256,
            data: alloy::sol_types::private::Bytes,
            predecessor: alloy::sol_types::private::FixedBytes<32>,
            salt: alloy::sol_types::private::FixedBytes<32>,
            delay: alloy::sol_types::private::primitives::aliases::U256,
        ) -> alloy_contract::SolCallBuilder<T, &P, scheduleCall, N> {
            self.call_builder(&scheduleCall {
                target,
                value,
                data,
                predecessor,
                salt,
                delay,
            })
        }
        ///Creates a new call builder for the [`scheduleBatch`] function.
        pub fn scheduleBatch(
            &self,
            targets: alloy::sol_types::private::Vec<alloy::sol_types::private::Address>,
            values: alloy::sol_types::private::Vec<
                alloy::sol_types::private::primitives::aliases::U256,
            >,
            payloads: alloy::sol_types::private::Vec<alloy::sol_types::private::Bytes>,
            predecessor: alloy::sol_types::private::FixedBytes<32>,
            salt: alloy::sol_types::private::FixedBytes<32>,
            delay: alloy::sol_types::private::primitives::aliases::U256,
        ) -> alloy_contract::SolCallBuilder<T, &P, scheduleBatchCall, N> {
            self.call_builder(&scheduleBatchCall {
                targets,
                values,
                payloads,
                predecessor,
                salt,
                delay,
            })
        }
        ///Creates a new call builder for the [`supportsInterface`] function.
        pub fn supportsInterface(
            &self,
            interfaceId: alloy::sol_types::private::FixedBytes<4>,
        ) -> alloy_contract::SolCallBuilder<T, &P, supportsInterfaceCall, N> {
            self.call_builder(&supportsInterfaceCall { interfaceId })
        }
        ///Creates a new call builder for the [`updateDelay`] function.
        pub fn updateDelay(
            &self,
            newDelay: alloy::sol_types::private::primitives::aliases::U256,
        ) -> alloy_contract::SolCallBuilder<T, &P, updateDelayCall, N> {
            self.call_builder(&updateDelayCall { newDelay })
        }
    }
    /// Event filters.
    #[automatically_derived]
    impl<
            T: alloy_contract::private::Transport + ::core::clone::Clone,
            P: alloy_contract::private::Provider<T, N>,
            N: alloy_contract::private::Network,
        > TimelockInstance<T, P, N>
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
        ///Creates a new event filter for the [`CallExecuted`] event.
        pub fn CallExecuted_filter(&self) -> alloy_contract::Event<T, &P, CallExecuted, N> {
            self.event_filter::<CallExecuted>()
        }
        ///Creates a new event filter for the [`CallSalt`] event.
        pub fn CallSalt_filter(&self) -> alloy_contract::Event<T, &P, CallSalt, N> {
            self.event_filter::<CallSalt>()
        }
        ///Creates a new event filter for the [`CallScheduled`] event.
        pub fn CallScheduled_filter(&self) -> alloy_contract::Event<T, &P, CallScheduled, N> {
            self.event_filter::<CallScheduled>()
        }
        ///Creates a new event filter for the [`Cancelled`] event.
        pub fn Cancelled_filter(&self) -> alloy_contract::Event<T, &P, Cancelled, N> {
            self.event_filter::<Cancelled>()
        }
        ///Creates a new event filter for the [`MinDelayChange`] event.
        pub fn MinDelayChange_filter(&self) -> alloy_contract::Event<T, &P, MinDelayChange, N> {
            self.event_filter::<MinDelayChange>()
        }
        ///Creates a new event filter for the [`RoleAdminChanged`] event.
        pub fn RoleAdminChanged_filter(&self) -> alloy_contract::Event<T, &P, RoleAdminChanged, N> {
            self.event_filter::<RoleAdminChanged>()
        }
        ///Creates a new event filter for the [`RoleGranted`] event.
        pub fn RoleGranted_filter(&self) -> alloy_contract::Event<T, &P, RoleGranted, N> {
            self.event_filter::<RoleGranted>()
        }
        ///Creates a new event filter for the [`RoleRevoked`] event.
        pub fn RoleRevoked_filter(&self) -> alloy_contract::Event<T, &P, RoleRevoked, N> {
            self.event_filter::<RoleRevoked>()
        }
    }
}
