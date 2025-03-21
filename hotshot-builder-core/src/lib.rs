// Copyright (c) 2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot Builder Protocol.
//

// Builder Phase 1
// It mainly provides three API services to hotshot proposers:
// 1. Serves a proposer(leader)'s request to provide blocks information
// 2. Serves a proposer(leader)'s request to provide the full blocks information
// 3. Serves a proposer(leader)'s request to provide the block header information

// It also provides one API services external users:
// 1. Serves a user's request to submit a private transaction

// providing the core services to support above API services
pub mod builder_state;

// Core interaction with the HotShot network
pub mod service;

// tracking the testing
#[cfg(test)]
pub mod testing;
