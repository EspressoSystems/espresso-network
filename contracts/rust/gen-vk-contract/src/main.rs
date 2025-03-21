//! This executable generates the solidity files with hardcoded verifying keys for
//! LightClient updates by running `cargo run -p gen-vk-contract --release`.
//! LightClientMock updates by running `cargo run -p gen-vk-contract --release -- --mock`.
//! Adapted from [CAPE project][https://github.com/EspressoSystems/cape/blob/main/contracts/rust/src/bin/gen-vk-libraries.rs]

use std::{fs::OpenOptions, io::Write, path::PathBuf, process::Command};

use clap::Parser;
use ethers::core::abi::AbiEncode;
use hotshot_contract_adapter::jellyfish::ParsedVerifyingKey;
use hotshot_stake_table::config::STAKE_TABLE_CAPACITY;
use jf_pcs::prelude::UnivariateUniversalParams;

#[derive(Parser)]
struct Cli {
    /// indicate if it's for the mock verification key
    #[arg(long, default_value_t = false)]
    mock: bool,
}

fn main() {
    let mock = Cli::parse().mock;

    let srs = {
        // load SRS from Aztec's ceremony
        let srs = if mock {
            ark_srs::kzg10::aztec20::setup(2u64.pow(16) as usize + 2)
                .expect("Aztec SRS fail to load")
        } else {
            ark_srs::kzg10::aztec20::setup(2u64.pow(20) as usize + 2)
                .expect("Aztec SRS fail to load")
        };
        // convert to Jellyfish type
        // TODO: (alex) use constructor instead https://github.com/EspressoSystems/jellyfish/issues/440
        UnivariateUniversalParams {
            powers_of_g: srs.powers_of_g,
            h: srs.h,
            beta_h: srs.beta_h,
            powers_of_h: vec![srs.h, srs.beta_h],
        }
    };
    let (_, vk) = if mock {
        hotshot_state_prover::preprocess(&srs, 10).expect("Circuit preprocess failed")
    } else {
        hotshot_state_prover::preprocess(&srs, STAKE_TABLE_CAPACITY)
            .expect("Circuit preprocess failed")
    };
    let vk: ParsedVerifyingKey = vk.into();

    // calculate the path to solidity file
    let contract_name = if mock {
        "LightClientStateUpdateVKMock"
    } else {
        "LightClientStateUpdateVK"
    };
    let mut path = PathBuf::new();
    path.push(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    if mock {
        path.push("test/mocks");
    } else {
        path.push("src/libraries");
    }
    path.push(contract_name);
    path.set_extension("sol");
    println!("Path:{:?}", path.to_str());

    // overwrite the file
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .unwrap();

    let import_path = if mock {
        "import { IPlonkVerifier } from \"../../src/interfaces/IPlonkVerifier.sol\";"
    } else {
        "import { IPlonkVerifier } from \"../interfaces/IPlonkVerifier.sol\";"
    };

    let code = format!(
                "// SPDX-License-Identifier: GPL-3.0-or-later
    //
    // Copyright (c) 2023 Espresso Systems (espressosys.com)
    // This file is part of the Espresso Sequencer project.
    //
    // This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
    // This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
    // You should have received a copy of the GNU General Public License along with this program. If not, see <https://www.gnu.org/licenses/>.

    // NOTE: DO NOT MODIFY! GENERATED BY SCRIPT VIA `cargo run --bin gen-vk-contract --release`.
    pragma solidity ^0.8.0;

    {}

    /* solhint-disable no-inline-assembly */

    library {} {{
        function getVk() internal pure returns (IPlonkVerifier.VerifyingKey memory vk) {{
            assembly {{
                // domain size
                mstore(vk, {})
                // num of public inputs
                mstore(add(vk, 0x20), {})

                // sigma0
                mstore(mload(add(vk, 0x40)), {})
                mstore(add(mload(add(vk, 0x40)), 0x20), {})
                // sigma1
                mstore(mload(add(vk, 0x60)), {})
                mstore(add(mload(add(vk, 0x60)), 0x20), {})
                // sigma2
                mstore(mload(add(vk, 0x80)), {})
                mstore(add(mload(add(vk, 0x80)), 0x20), {})
                // sigma3
                mstore(mload(add(vk, 0xa0)), {})
                mstore(add(mload(add(vk, 0xa0)), 0x20), {})
                // sigma4
                mstore(mload(add(vk, 0xc0)), {})
                mstore(add(mload(add(vk, 0xc0)), 0x20), {})

                // q1
                mstore(mload(add(vk, 0xe0)), {})
                mstore(add(mload(add(vk, 0xe0)), 0x20), {})
                // q2
                mstore(mload(add(vk, 0x100)), {})
                mstore(add(mload(add(vk, 0x100)), 0x20), {})
                // q3
                mstore(mload(add(vk, 0x120)), {})
                mstore(add(mload(add(vk, 0x120)), 0x20), {})
                // q4
                mstore(mload(add(vk, 0x140)), {})
                mstore(add(mload(add(vk, 0x140)), 0x20), {})

                // qM12
                mstore(mload(add(vk, 0x160)), {})
                mstore(add(mload(add(vk, 0x160)), 0x20), {})
                // qM34
                mstore(mload(add(vk, 0x180)), {})
                mstore(add(mload(add(vk, 0x180)), 0x20), {})

                 // qO
                mstore(mload(add(vk, 0x1a0)), {})
                mstore(add(mload(add(vk, 0x1a0)), 0x20), {})
                 // qC
                mstore(mload(add(vk, 0x1c0)), {})
                mstore(add(mload(add(vk, 0x1c0)), 0x20), {})
                 // qH1
                mstore(mload(add(vk, 0x1e0)), {})
                mstore(add(mload(add(vk, 0x1e0)), 0x20), {})
                 // qH2
                mstore(mload(add(vk, 0x200)), {})
                mstore(add(mload(add(vk, 0x200)), 0x20), {})
                 // qH3
                mstore(mload(add(vk, 0x220)), {})
                mstore(add(mload(add(vk, 0x220)), 0x20), {})
                 // qH4
                mstore(mload(add(vk, 0x240)), {})
                mstore(add(mload(add(vk, 0x240)), 0x20), {})
                 // qEcc
                mstore(mload(add(vk, 0x260)), {})
                mstore(add(mload(add(vk, 0x260)), 0x20), {})
                 // g2LSB
                mstore(add(vk, 0x280), {})
                 // g2MSB
                mstore(add(vk, 0x2A0), {})
            }}
        }}
    }}",
    import_path,
                contract_name,
                vk.domain_size,
                vk.num_inputs,
                vk.sigma_0.x,
                vk.sigma_0.y,
                vk.sigma_1.x,
                vk.sigma_1.y,
                vk.sigma_2.x,
                vk.sigma_2.y,
                vk.sigma_3.x,
                vk.sigma_3.y,
                vk.sigma_4.x,
                vk.sigma_4.y,
                vk.q_1.x,
                vk.q_1.y,
                vk.q_2.x,
                vk.q_2.y,
                vk.q_3.x,
                vk.q_3.y,
                vk.q_4.x,
                vk.q_4.y,
                vk.q_m_12.x,
                vk.q_m_12.y,
                vk.q_m_34.x,
                vk.q_m_34.y,
                vk.q_o.x,
                vk.q_o.y,
                vk.q_c.x,
                vk.q_c.y,
                vk.q_h_1.x,
                vk.q_h_1.y,
                vk.q_h_2.x,
                vk.q_h_2.y,
                vk.q_h_3.x,
                vk.q_h_3.y,
                vk.q_h_4.x,
                vk.q_h_4.y,
                vk.q_ecc.x,
                vk.q_ecc.y,
                vk.g2_lsb.encode_hex(),
                vk.g2_msb.encode_hex(),
            )
            .into_bytes();

    file.write_all(&code).unwrap();

    // format the contract
    Command::new("just")
        .arg("sol-lint")
        .output()
        .expect("Failed to lint the contract code");
}
