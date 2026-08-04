# Reference Data

This directory contains reference instantiations of the data types used by the Espresso node which have a stable
language-agnostic interface for serialization (in both `.json` files and binary `.bin` files) and cryptographic
commitments. The objects in this directory have well-known commitments. They serve as examples of the data formats used
by Espresso Network, and can be used as test cases for ports of the serialization and commitment algorithms to other
languages.

The Rust module `espresso-types::reference_tests` contains test cases which are designed to fail if the serialization
format or commitment scheme for any of these data types changes. If you make a breaking change, you may need to update
these reference objects as well. Running those tests will also print out information about the commitments of these
reference objects, which can be useful for generating test cases for ports. To run them and get the output, use

```bash
cargo test --all-features -p espresso-types -- --nocapture --test-threads 1 reference_tests
```

Vectors are grouped by protocol version, so `data/vN` holds the objects as they are serialized at version `0.N`.
Alongside the types above, each version directory may contain message vectors, which pin the consensus wire format
rather than a single data type:

- `messages.{json,bin}` is the HotShot `Message` envelope, produced by `espresso-node::message_compat_tests`.
- `new_protocol_messages.{json,bin}` is the new protocol (fast finality) `Message` envelope introduced at version 0.6,
  produced by `espresso-node::new_protocol_message_compat_tests`.

Binary message vectors are serialized exactly as nodes serialize them on the wire, so they begin with a four byte
version prefix.

```bash
cargo test -p espresso-node message_compat
```
