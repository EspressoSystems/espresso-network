# Espresso Types

This crate provides the data types that make up the Espresso Network, along with the logic that defines
how these types interact and evolve as the network operates. It also provides a versioning system that enables network
upgrades, including changes to data types, that maintains adequate compatibility with older types so that newer versions
of the software are able to interpret an Espresso blockchain all the way back to genesis.

## Design Principles

### Compatibility Within Reason

Blockchains have the unique problem of needing to maintain backwards compatibility with every previous version of the
protocol, so that old data can be interpreted and replayed as necessary to derive the current blockchain state. Thus, it
is highly desirable to have one set of types at any given time, which is backwards compatible with all older shipped
versions, and to minimize differences between versions so that we can avoid as much as possible enum types and
conditional logic to handle different versions.

To the extent that differences between versions are minimized, it is practical to maintain one codebase with full
backwards compatibility, with only minor conditionals limited in scope. Due to this strong compatibility, changes made
in this manner -- that is, affecting some logic but maintaining one coherent set of types and backwards serialization
compatibility -- correspond to minor version changes.

Over time, it is possible that these minor changes will accumulate to the point where it is infeasible to handle all the
various cases in one set of code. Or, a significant protocol upgrade might make it impractical to maintain backwards
compatibility using a single set of types and logic. In this case, a _major_ version increment may be necessary, where
we create a new set of types and logic with a fresh slate and no backwards compatibility burden. In such cases,
applications that use this crate (e.g. consensus, archival query service) will be responsible for switching between two
sets of types (e.g. major versions 1 and 2) as necessary, depending on what part of the history of the blockchain they
are dealing with.

### Separation of Data from Code

Due to the constraints of serialization, and specifically the desirability of maintaining `serde` compatibility as much
as possible, the most practical way to handle different versions of data is to have independent, parallel definitions of
the data types for each supported version. These definitions exist in their own namespaces within this crate, such as
`v0_1::Header` for the `Header` type from version 0.1, `v0_2::Header`, etc.

Code, on the other hand, benefits from being as unified as possible. Having entirely separate implementations for each
version would make it harder to spot differences and similarities between versions visually, increase the burden of
maintenance and testing, and lead to large amounts of duplicate code where logic hasn't changed between versions (or
else a confusing mess of slightly customizable helper functions shared across versions).

As such, for each _major_ version, there is one implementation of the network logic that encompasses all minor versions.
Each major version defines top-level types like `v0::Header` which are compatible across that entire major version. For
example, `v0::Header` implements `From<v0_1::Header>` and `From<v0_2::Header>`. Its serialization will output the
appropriate minor version format depending on which minor version was used to construct the header, and it implements
`deserialize_as(Version)` which interprets the input as the specified format version.

This major version compatibility header implements all of the network logic for all minor versions within its major
version; operations on headers and states will follow the logic for the minor version which was used to construct the
header.

## Repository Structure

The repository is divided into top-level modules for each supported major version. All types from the most recent major
version are also re-exported from the top level of the crate. This allows applications which intend to stay up-to-date
with the latest types to import directly from the top level, and then a simple `cargo update` is sufficient to bring in
the latest types, at which point the application can be updated as necessary. Meanwhile, applications that intend to pin
to a specific stable major version can import the versioned types from the appropriate module.

The structure of each major version module mirrors the top level structure recursively. There are sub-modules for each
minor version within that major version, and the latest types for that major version are reexported from the major
version module itself.

Note that the minor version sub-modules _only_ define data structures and derivable trait implementations (such as
`Debug` and `serde` traits). All operations on these data structures, including constructors and field accessors, are
defined in the major version module. This upholds design principle 2, by separating the versioned data structure layouts
from the version-agnostic Rust interfaces we use to deal with these data structures.

Each major version module also contains a `traits` submodule containing implementations of HotShot traits for the types
for that major version, allowing them to be used to instantiate HotShot consensus and related applications, like the
query service.

## Conventions and Best Practices

### Use re-exports to minimize duplicated data structures

Data structures that have not changed from one minor version to the next can be re-exported from the previous minor
version. E.g. in `v0::v0_2`, we might have `pub use super::v0_1::ChainConfig` if the `ChainConfig` type has not changed
between these two versions.

Data structures that have not changed across any minor version within a major version can be re-exported in the major
version module from the latest minor version, but a static assertion must be present checking that the re-exported type
is the same type as exported from each of the minor version modules, e.g.

```rust
pub use v0_2::ChainConfig;

static_assert_unchanged_type!(ChainConfig);
```

### All fields are private

The goal of each major version is to provide a consistent Rust interface that works regardless of which minor version is
being used for the underlying data structure. To achieve this while allowing changes in the data layout, all fields
should be private (or `pub(crate)`). All consumers of this crate should access the data via public methods defined in
the major version module, since the implementation of these methods can often be changed without changing the interface
in case the data layout changes.

### Unversioned types considered code

The pain of maintaining parallel sets of versioned types means we should only do it when absolutely necessary: for
serializable types that are used either as consensus messages or persistent storage, or for types used to define such
types.

Other types which are used only as part of the Rust API, as transient, in-memory types, should be defined alongside
implementations and treated as part of code, not data. An example is the `EthKeyPair` type, which is only used as a
convenient wrapper to hold a public and private key pair, but does not appear as part of any serialized data structure.
