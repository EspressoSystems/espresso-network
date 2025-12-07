# Smart Contract Upgrades

Code on the EVM is immutable but via proxies, smart contracts can be “upgraded”. A proxy contract acts as an
intermediary that can delegate calls to other contracts, called implementation contracts. This allows the logic of a
smart contract to be upgraded while keeping the same address and state. This is made possible by two functions in
Solidity:

- `delegatecall`: ContractA can delegatecall ContractB but execute the logic within the context of ContractA. This means
  that the called contract's code is executed with the storage, caller, and balance of the calling contract.
- `fallback`: executed when a function does not exist. This is often used in proxy contracts to catch all function calls
  and delegate them to the implementation contract.

OpenZeppelin's [implementation](https://docs.openzeppelin.com/contracts/4.x/api/proxy#UUPSUpgradeable) for the UUPS
proxy pattern was used.

At the time of writing, we have two upgradable smart contracts:

- LightClient.sol
- FeeContract.sol

They both use OpenZeppelin's [implementation](https://docs.openzeppelin.com/contracts/4.x/api/proxy#UUPSUpgradeable) for
the UUPS proxy pattern and is deployed using OpenZeppelin Upgrades library.

## Writing Upgradable Smart Contracts

This document is intended for developers to explain the process for upgrading contracts within our repo to ensure safe
upgrades.

### Upgrading via Contract Inheritance

When writing the upgrade for upgradable contracts such as the light client contract and the fee contract, the new
implementation inherits from the latest implementation contract. This new implementation only implements the new or
modified features or new variables. If any variables have to be initialized, the new implementation contract should
create a `initializeVx` function, where x is the version number.

The `reinitializer` modifier, provided by the
[`Initializable`](https://github.com/OpenZeppelin/openzeppelin-contracts/blob/master/contracts/proxy/utils/Initializable.sol)
library, ensures that the new implementation is reinitialized at most once. The modifier takes a `uint64` argument,
which specifies the contract's version.

When the base implementation contract is initialized, its `_initializedVersion` is set to 1. The `reinitializer`
modifier ensures that the initialization logic is only executed if the \_initializedVersion is less than the specified
version number. Therefore, when upgrading a contract, the `reinitializer` modifier should be set to the previous
`_initializedVersion` plus 1 to indicate the new version.

Here's an example:

The original `LightClient` contract looks like this:

```solidity
contract LightClient is Initializable, OwnableUpgradeable, UUPSUpgradeable {
  //...
}
```

The 2nd implementation, LightClientV2, that adds a new variable is implemented like this:

```solidity
contract LightClientV2 is LightClient {
    uint256 public newField;

    /// @notice Initialize v2
    /// @param _newField   New field amount
    /// @dev the reinitializer modifier is used to reinitialize new versions of a contract and
    /// is called at most once. The modifier has an uint64 argument which indicates the next contract version.
    /// when the base implementation contract is initialized for the first time, the _initialized version
    /// is set to 1. Since this is the 2nd implementation, the next contract version is 2.
    function initializeV2(uint256 _newField) external reinitializer(2){
        newField = _newField;
    }

    function newFinalizedState(
        LightClientState memory newState,
        IPlonkVerifier.PlonkProof memory proof
    ) external virtual override {
      //...
    }

    //...
}
```

The third implementation, LightClientV3, will inherit from LightClientV2. Ensure that functions are marked as `virtual`
in previous versions of the contract if you intend to override them in child contracts.

```solidity
contract LightClientV3 is LightClientV2 {
    uint256 public anotherField;

    /// @param _newField   New field amount
    function initializeV3(uint256 _newField) external reinitializer(3){
        anotherField = _newField;
    }

    function newFinalizedState(
        LightClientState memory newState,
        IPlonkVerifier.PlonkProof memory proof
    ) external virtual override {
      //...
    }

    //...
}
```

Solidity supports inheritance which means that a function call always executes the function of the same name (and
parameter types) in the most derived contract in the inheritance hierarchy.

**Pros**:

- **Modularity**: each contract version can focus on specific features or upgrades
- **Reusability:** code doesn’t have to be duplicated in newer versions which reduces the likelihood of errors
- **Ease of extension:** easier to extend features without worrying about the rest of the implementation
- **C3 linearization**: C3 linearization provides a deterministic way to determine the storage layout of a contract
  based on its inheritance hierarchy. This means that if we maintain the same inheritance order in subsequent contract
  versions, the storage layout will remain consistent, even if we add new state variables.

**Cons**:

- **Deployment size:** Contracts that inherit from multiple base contracts can become large, potentially exceeding the
  maximum contract size limit on the Ethereum blockchain
- **State variable shadowing:** When a new version of a contract inherits from an older version, introducing a state
  variable with the same name as one in the base contract will lead to a compilation error due to shadowing. Therefore,
  one cannot simply redefine an existing struct with new members if inheritance is used for upgrades. This can become
  cumbersome in large projects with many inherited contracts
- **Multiple inheritance:** Solidity’s C3 linearization predicts the order in which base contracts are inherited, and
  their functions are called. So the order inherited is the order they will be in the storage layout and this needs to
  be taken under consideration. When multiple base contracts define state variables, the order of inheritance can affect
  the storage layout. Mismanaging this can lead to overlapping storage slots, causing data corruption.

### Upgrades supported

- new state variables
- new logic
- modifying existing logic (function overriding)

#### New State Variables

Thanks to C3 linearization, the order in which you inherit from the previous version of the contract dictates the
storage layout so be sure to be mindful when declaring new variables.

Criteria for New State Variables:

- New variable names must be used otherwise it leads to a compilation error due to shadowing.
- New variables added _must_ be declared as type `internal` or `public`, `private` can never be used.
- If the new variable needs to be initialized, then the initialize function with a separate initV[X] function should be
  added. The initializeVx function allows the contract to initialize only the new variables added in the new contract.

_Example:_

```solidity
// Version 1
contract V1 {
    uint public oldValue;
    uint8 internal _initializedVersion

    function initialize(uint _oldValue) external {
        require(_initializedVersion == 0);
        oldValue = _oldValue;
        _initializedVersion = 1;
    }
}

// Version 2
contract V2 is V1 {
    uint public newValue;

    function initializeV2(uint _newValue) external {
        require(_initializedVersion == 1);
        newValue = _newValue;
        _initializedVersion = 2;
    }
}
```

#### New Logic

Base functions can be overridden by inheriting contracts to change their behavior if they are marked as virtual. Thus if
you plan to keep a function upgradable, always mark it as virtual in new versions of the contract.

_Example:_

```solidity
// Version 1
contract V1 {
    function getValue() public view virtual returns (uint) {
        return 1;
    }
}

// Version 2
contract V2 is V1 {
    function getValue() public view virtual override returns (uint) {
        return 2;
    }
}
```

#### Modifying Existing Logic (Function Overriding)

The logic in existing functions can be overridden but be sure not to introduce breaking changes.

Criteria for Overriding Functions::

- a function must be marked as `virtual` in the base contract so that it can be eligible to be overridden in a derived
  contract.
- a function with `private` visibility cannot be overridden or inherited.
- Public state variables can override `external` functions if the parameter and return types of the function matches the
  getter function of the variable:

_Example:_

```solidity
// SPDX-License-Identifier: GPL-3.0
pragma solidity >=0.6.0 <0.9.0;

contract A {
     function getValue() public view virtual returns (uint) {
        return 1;
    }
}

contract B is A {
    function getValue() public view virtual override returns (uint) {
        return 2;
    }
}
```

- The overriding function may only change the visibility of the overridden function from `external` to `public`.
- The mutability may be changed to a more strict one following the order: `nonpayable` can be overridden by `view` and
  `pure`. `view` can be overridden by `pure`. `payable` is an exception and cannot be changed to any other mutability.

##### Avoiding breaking changes when overriding functions

- When overriding a function, consider whether it’s necessary to call the parent contract’s implementation using
  `super`. This can preserve the original logic as you extend with new functionality.
- Ensure that any side effects are handled appropriately, e.g. event emissions, state variable modifications etc

### Upgrades that become complicated through inheritance

#### Upgrading Structs

Unfortunately, directly upgrading structs through inheritance is not straightforward due to Solidity's storage layout
rules. When a new version of a contract inherits from an older version, introducing a state variable with the same name
as one in the base contract will lead to a compilation error due to shadowing.

**Potential workarounds:**

- Additional struct: create a new struct with additional fields that extends the original struct so that both can be
  used in tandem.
- New Struct: Create a new struct with additional fields and migrate existing data to the new struct during the upgrade
  process. _Example:_

```solidity

// Version 1
contract V1 {
    struct Data {
        uint value;
    }

    Data public data;
}

// Version 2
contract V2 is V1 {
    struct NewData {
        uint value;
        uint newValue;
    }

    NewData public newData;

    function migrateData() public {
        newData = NewData(data.value, 0);
    }
}
```

## Deploying the Upgradable Contract

The [OpenZeppelin Upgrade plugins](https://docs.openzeppelin.com/upgrades-plugins) are used to assist with upgrades
so that we can ensure that the base and future implementation contracts are upgrade safe. For example, the
implementation contract:

- cannot have a constructor.
- should not use the `selfdestruct` or `delegatecall`.
- should not initialize state variables.
- should not make state variables `immutable`.

You can learn more about deploying and upgrading upgradable contracts with OpenZeppelin and Foundry
[here](https://docs.openzeppelin.com/upgrades-plugins/1.x/foundry-upgrades).

### Upgrading via the Proxy

- Deploy the new (modified) implementation contract. If new state variables that need initialization were added then:
- Prepare an `upgradeToAndCall` transaction from the multisig admin account which is parameterized with the address of
  the new implementation contract and an internal call to invoke initializeVx with the new state variables data. Else:
- Prepare an `upgradeTo` transaction from the multisig admin account parameterized with the address of the new
  implementation contract: Then:
- Broadcast the transaction

For more details, check out this [README](../contracts/script/README.md).
