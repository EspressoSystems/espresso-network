//! Compile-only probe for `just check-sp1-target`: keeps `espresso-types` and
//! `light-client` building for the SP1 zkVM target the way a guest program
//! consumes them. See `doc/cargo-features.md`.

use espresso_types as _;
use getrandom as _;
use light_client as _;
