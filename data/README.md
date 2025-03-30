# Reference Data

This directory contains reference instantiations of the data types used by the network which have a stable
language-agnostic interface for serialization (in both `.json` files and binary `.bin` files) and cryptographic
commitments. The objects in this directory have well-known commitments. They serve as examples of the data formats used
by the Espresso Network, and can be used as test cases for ports of the serialization and commitment algorithms to
other languages.

The Rust module `espresso-types::reference_tests` contains test cases which are designed to fail if the serialization
format or commitment scheme for any of these data types changes. If you make a breaking change, you may need to update
these reference objects as well. Running those tests will also print out information about the commitments of these
reference objects, which can be useful for generating test cases for ports. To run them and get the output, use

```bash
cargo test --all-features -p espresso-types -- --nocapture --test-threads 1 reference_tests
```
