use std::io::Read;

use alloy::{
    node_bindings::Anvil,
    providers::{ext::AnvilApi, ProviderBuilder},
};
use assert_cmd::Command;
use espresso_contract_deployer::{Contract, DeploymentState};
use flate2::read::GzDecoder;
use predicates::str;
use serde_json::Value;
use tempfile::NamedTempFile;

const DEPLOYER_ADDRESS: &str = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";

fn expected_contracts_after_step(step: usize) -> Vec<Contract> {
    let all_steps = [
        vec![Contract::OpsTimelock],
        vec![Contract::SafeExitTimelock],
        vec![Contract::FeeContract, Contract::FeeContractProxy],
        vec![Contract::EspToken, Contract::EspTokenProxy],
        vec![
            Contract::PlonkVerifier,
            Contract::LightClient,
            Contract::LightClientProxy,
        ],
        vec![Contract::PlonkVerifierV2, Contract::LightClientV2],
        vec![Contract::PlonkVerifierV3, Contract::LightClientV3],
        vec![Contract::RewardClaim, Contract::RewardClaimProxy],
        vec![Contract::EspTokenV2],
        vec![Contract::StakeTable, Contract::StakeTableProxy],
        vec![Contract::StakeTableV2],
    ];

    all_steps.iter().take(step).flatten().cloned().collect()
}

#[allow(deprecated)]
fn deploy_cmd(rpc_url: &str, state_path: &str) -> Command {
    let mut cmd = Command::cargo_bin("deploy").unwrap();
    cmd.arg("--rpc-url")
        .arg(rpc_url)
        .arg("--state-file")
        .arg(state_path)
        .arg("--mock-espresso-live-network")
        .args(["--ops-timelock-admin", DEPLOYER_ADDRESS])
        .args(["--ops-timelock-delay", "0"])
        .args(["--ops-timelock-executors", DEPLOYER_ADDRESS])
        .args(["--ops-timelock-proposers", DEPLOYER_ADDRESS])
        .args(["--safe-exit-timelock-admin", DEPLOYER_ADDRESS])
        .args(["--safe-exit-timelock-delay", "0"])
        .args(["--safe-exit-timelock-executors", DEPLOYER_ADDRESS])
        .args(["--safe-exit-timelock-proposers", DEPLOYER_ADDRESS])
        .args(["--initial-token-grant-recipient", DEPLOYER_ADDRESS])
        .args(["--token-name", "Espresso"])
        .args(["--token-symbol", "ESP"])
        .args(["--initial-token-supply", "1000000000000000000000000"]);
    cmd
}

fn deploy_all_steps(rpc_url: &str, state_path: &str) {
    let mut iteration = 0;
    loop {
        iteration += 1;
        let assert = deploy_cmd(rpc_url, state_path)
            .arg("--one-step")
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        if stdout.contains("All steps complete") {
            assert.stdout(str::contains("All steps complete"));
            let state = DeploymentState::load(state_path).expect("Failed to load state");
            assert_eq!(state.contracts.len(), 19);
            break;
        }

        let state = DeploymentState::load(state_path).expect("Failed to load state");
        let expected = expected_contracts_after_step(iteration);
        for contract in &expected {
            assert!(state.contracts.contains_key(contract));
        }
    }
}

fn get_anvil_accounts(dump: &[u8]) -> Value {
    let mut decoder = GzDecoder::new(dump);
    let mut json_str = String::new();
    decoder.read_to_string(&mut json_str).unwrap();
    let state: Value = serde_json::from_str(&json_str).unwrap();
    state["accounts"].clone()
}

#[test_log::test(tokio::test)]
async fn test_deploy_determinism() {
    let first_anvil = Anvil::new().spawn();
    let first_rpc_url = first_anvil.endpoint();

    let first_state_file = NamedTempFile::new().unwrap();
    let first_state_path = first_state_file.path().to_str().unwrap();

    deploy_all_steps(&first_rpc_url, first_state_path);

    let first_provider = ProviderBuilder::new()
        .connect(&first_rpc_url)
        .await
        .unwrap();
    let first_accounts = get_anvil_accounts(&first_provider.anvil_dump_state().await.unwrap());

    // Deploy again on fresh anvil, verify same accounts state
    let second_anvil = Anvil::new().spawn();
    let second_rpc_url = second_anvil.endpoint();

    let second_state_file = NamedTempFile::new().unwrap();
    let second_state_path = second_state_file.path().to_str().unwrap();

    deploy_all_steps(&second_rpc_url, second_state_path);

    let second_provider = ProviderBuilder::new()
        .connect(&second_rpc_url)
        .await
        .unwrap();
    let second_accounts = get_anvil_accounts(&second_provider.anvil_dump_state().await.unwrap());

    // Find differences
    let first_obj = first_accounts.as_object().unwrap();
    let second_obj = second_accounts.as_object().unwrap();

    for (addr, first_val) in first_obj {
        if let Some(second_val) = second_obj.get(addr) {
            if first_val != second_val {
                let first_acc = first_val.as_object().unwrap();
                let second_acc = second_val.as_object().unwrap();
                println!("Account {addr} differs:");
                if first_acc.get("balance") != second_acc.get("balance") {
                    println!(
                        "  balance: first={} second={}",
                        first_acc.get("balance").unwrap(),
                        second_acc.get("balance").unwrap()
                    );
                }
                if first_acc.get("nonce") != second_acc.get("nonce") {
                    println!(
                        "  nonce: first={} second={}",
                        first_acc.get("nonce").unwrap(),
                        second_acc.get("nonce").unwrap()
                    );
                }
                if first_acc.get("code") != second_acc.get("code") {
                    println!(
                        "  code differs (lengths: first={} second={})",
                        first_acc.get("code").unwrap().as_str().unwrap().len(),
                        second_acc.get("code").unwrap().as_str().unwrap().len()
                    );
                }
                if first_acc.get("storage") != second_acc.get("storage") {
                    let first_storage = first_acc.get("storage").unwrap().as_object().unwrap();
                    let second_storage = second_acc.get("storage").unwrap().as_object().unwrap();
                    for (slot, first_v) in first_storage {
                        if second_storage.get(slot) != Some(first_v) {
                            println!(
                                "  storage[{slot}]: first={first_v} second={:?}",
                                second_storage.get(slot)
                            );
                        }
                    }
                    for (slot, second_v) in second_storage {
                        if !first_storage.contains_key(slot) {
                            println!("  storage[{slot}]: first=None second={second_v}");
                        }
                    }
                }
            }
        } else {
            println!("Account {addr} only in first");
        }
    }
    for addr in second_obj.keys() {
        if !first_obj.contains_key(addr) {
            println!("Account {addr} only in second");
        }
    }

    assert_eq!(first_accounts, second_accounts);
}
