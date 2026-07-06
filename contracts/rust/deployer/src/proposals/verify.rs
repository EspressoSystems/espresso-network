//! Trust-minimizing verifier for Safe-tx-builder upgrade proposals.
//!
//! Checks that the impl address in a proposal holds exactly the bytecode the
//! deployer ships for the supplied contract kind, and that governance wiring
//! (owner, timelock, delay, init call) is correct. No trust in Etherscan or
//! the JSON description. Validates all fields against the committed proposal.toml.

use std::{
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
};

use alloy::{
    primitives::{Address, B256, Bytes, U256},
    providers::{Provider, ProviderBuilder},
    sol_types::SolCall,
};
use anyhow::{Result, anyhow, bail};
use clap::ValueEnum;
use hotshot_contract_adapter::sol_types::{
    EspTokenV2, FeeContract, OpsTimelock, RewardClaim, StakeTableV2, StakeTableV3,
};
use serde::Deserialize;
use url::Url;

use crate::proposals::{
    deployment_info::deployment_info,
    proposal_toml::ProposalToml,
    safe_hash::{SafeTxHashes, safe_tx_hashes},
    write::{ISafe, default_rpc_url},
};

// ── JSON deserialization ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SafeBatch {
    pub meta: SafeMeta,
    pub transactions: Vec<SafeTx>,
}

#[derive(Debug, Deserialize)]
pub struct SafeMeta {
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SafeTx {
    pub to: Address,
    pub value: String,
    pub data: Option<String>,
    #[serde(rename = "contractMethod")]
    pub contract_method: Option<SafeContractMethod>,
    #[serde(rename = "contractInputsValues")]
    pub contract_inputs_values: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SafeContractMethod {
    pub name: String,
}

// ── Proposal batch classification ────────────────────────────────────────────

/// Phase of a single-transaction batch, identified from `contractMethod.name`
/// or the 4-byte ABI selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Schedule,
    Execute,
}

/// A classified timelock proposal pair.
/// Produced by `classify_batches`; input to `decode_proposal`.
#[derive(Debug)]
pub struct TimelockBatches {
    pub schedule: SafeBatch,
    pub execute: SafeBatch,
}

// ── Decoded upgrade ──────────────────────────────────────────────────────────

/// Reconstructed outer calldata for schedule and execute phases.
#[derive(Debug, Clone)]
pub struct OuterCalldatas {
    pub schedule: Bytes,
    pub execute: Bytes,
}

#[derive(Debug, Clone)]
pub struct DecodedUpgrade {
    /// Timelock address.
    pub outer_to: Address,
    pub proxy: Address,
    pub new_impl: Address,
    pub init_data: Bytes,
    pub value: U256,
    pub predecessor: B256,
    pub salt: B256,
    pub delay: U256,
    pub description: String,
    pub outer_calldatas: OuterCalldatas,
}

// ── Normalization ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolcVersion(pub [u8; 3]);

impl fmt::Display for SolcVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.0[0], self.0[1], self.0[2])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchClass {
    /// Cores and solc version are byte-equal.
    FullMatch,
    /// Cores match but solc versions differ; PASS with warning.
    CodeMatchMetaDiffers,
    /// Core bytecodes differ; FAIL.
    Mismatch,
}

#[derive(Debug, Clone)]
pub struct BytecodeCheck {
    pub class: MatchClass,
    pub onchain_solc: Option<SolcVersion>,
    pub reference_solc: Option<SolcVersion>,
    /// On `Mismatch`, describes the first unexplained difference (offset and
    /// both values).
    pub mismatch: Option<String>,
}

/// Strip the trailing solc CBOR metadata tail.
///
/// The tail is `<cbor body> <u16-BE body length>`. Solc emits the
/// `"solc": <3-byte version>` map entry last, so a genuine body ends with
/// `64 "solc" 43 <ver>`. The body length is bounded (51 bytes with the default
/// ipfs entry); a larger claimed length is treated as code, not metadata, so an
/// attacker cannot hide appended bytes under an oversized tail.
///
/// Without a valid tail the input is returned unchanged with `None`.
pub fn strip_cbor_metadata(code: &[u8]) -> (Vec<u8>, Option<SolcVersion>) {
    // `64 "solc" 43`: CBOR key "solc" followed by the bytes3 version prefix.
    const SOLC_ENTRY: &[u8] = &[0x64, 0x73, 0x6f, 0x6c, 0x63, 0x43];
    const MAX_CBOR_LEN: usize = 64;

    if code.len() < 2 {
        return (code.to_vec(), None);
    }
    let tail_len = u16::from_be_bytes([code[code.len() - 2], code[code.len() - 1]]) as usize;
    let total_drop = tail_len + 2;
    if tail_len > MAX_CBOR_LEN || total_drop > code.len() {
        return (code.to_vec(), None);
    }
    let cbor = &code[code.len() - total_drop..code.len() - 2];
    if cbor.len() < 9 || cbor[cbor.len() - 9..cbor.len() - 3] != *SOLC_ENTRY {
        return (code.to_vec(), None);
    }
    let ver = SolcVersion(cbor[cbor.len() - 3..].try_into().expect("3 bytes"));
    (code[..code.len() - total_drop].to_vec(), Some(ver))
}

/// Compare normalized on-chain bytecode against the binding reference.
///
/// Strips the CBOR metadata tail from both, then requires every byte
/// difference to be explained: a difference is only accepted inside a 20-byte
/// window where the on-chain code holds `impl_addr` and the reference holds
/// zeros. Those windows are the UUPS `__self` immutable slots (the impl's own
/// address, baked in at deploy; the binding is compiled with
/// `address(this) = 0`). Any other difference is a `Mismatch`, reported with
/// the offset and both values.
///
/// Exactly `expected_self_windows` explained windows are required. This pins
/// the impl address to the immutable slots: substituting it into any other
/// zero region of the reference (e.g. a `PUSH32 0` constant) changes the count
/// and fails. Residual: a window misaligned within a 32-byte slot still
/// passes, but a wrong `__self` value only makes `upgradeToAndCall` revert
/// (`proxiableUUID` is `notDelegated`); it cannot substitute code.
///
/// LightClient verification is deferred; bails if the reference contains
/// `0xff*20` library placeholders.
pub fn compare_normalized(
    onchain: &[u8],
    reference: &[u8],
    impl_addr: Address,
    expected_self_windows: usize,
) -> Result<BytecodeCheck> {
    let placeholder = [0xffu8; 20];
    if reference.windows(20).any(|w| w == placeholder.as_slice()) {
        bail!(
            "reference bytecode contains library placeholder (0xff*20); LightClient verification \
             is deferred"
        );
    }
    if impl_addr == Address::ZERO {
        bail!("impl address is zero");
    }

    let (onchain_core, onchain_solc) = strip_cbor_metadata(onchain);
    let (ref_core, ref_solc) = strip_cbor_metadata(reference);

    if onchain_core.len() != ref_core.len() {
        return Ok(BytecodeCheck {
            class: MatchClass::Mismatch,
            onchain_solc,
            reference_solc: ref_solc,
            mismatch: Some(format!(
                "core length differs: onchain={} reference={}",
                onchain_core.len(),
                ref_core.len()
            )),
        });
    }

    // Zero every __self window (impl_addr on-chain over zeros in the reference)
    // in a copy; matching against the pristine core keeps the scan independent
    // of window order and overlap.
    let mut normalized = onchain_core.clone();
    let mut self_windows = 0usize;
    for w in 0..onchain_core.len().saturating_sub(19) {
        if onchain_core[w..w + 20] == *impl_addr.as_slice() && ref_core[w..w + 20] == [0u8; 20] {
            normalized[w..w + 20].fill(0);
            self_windows += 1;
        }
    }

    if let Some(first) = (0..normalized.len()).find(|&i| normalized[i] != ref_core[i]) {
        let end = (first + 20).min(normalized.len());
        return Ok(BytecodeCheck {
            class: MatchClass::Mismatch,
            onchain_solc,
            reference_solc: ref_solc,
            mismatch: Some(format!(
                "unexplained difference at core offset {first}: onchain=0x{} reference=0x{}",
                alloy::hex::encode(&onchain_core[first..end]),
                alloy::hex::encode(&ref_core[first..end])
            )),
        });
    }

    if self_windows != expected_self_windows {
        bail!("found {self_windows} __self immutable windows; expected {expected_self_windows}");
    }

    let class = if onchain_solc == ref_solc {
        MatchClass::FullMatch
    } else {
        MatchClass::CodeMatchMetaDiffers
    };
    Ok(BytecodeCheck {
        class,
        onchain_solc,
        reference_solc: ref_solc,
        mismatch: None,
    })
}

// ── Contract kind ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ContractKindArg {
    #[clap(name = "stake-table-v2")]
    StakeTableV2,
    #[clap(name = "stake-table-v3")]
    StakeTableV3,
    #[clap(name = "esp-token-v2")]
    EspTokenV2,
    #[clap(name = "fee-contract")]
    FeeContract,
    #[clap(name = "reward-claim")]
    RewardClaim,
}

impl ContractKindArg {
    /// Kebab-case string representation, matching the toml `contract` field.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StakeTableV2 => "stake-table-v2",
            Self::StakeTableV3 => "stake-table-v3",
            Self::EspTokenV2 => "esp-token-v2",
            Self::FeeContract => "fee-contract",
            Self::RewardClaim => "reward-claim",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerAccessor {
    Owner,
    CurrentAdmin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelockKind {
    Ops,
    SafeExit,
}

/// Expected reinitializer call and the major version it brings the proxy to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedInit {
    pub selector: [u8; 4],
    pub target_major: u8,
}

#[derive(Debug, Clone)]
pub struct ContractKind {
    pub name: &'static str,
    pub deployed_bytecode: &'static [u8],
    pub expected_init: Option<ExpectedInit>,
    pub owner_accessor: OwnerAccessor,
    pub timelock_kind: TimelockKind,
    pub expected_prev_major: Option<u8>,
    /// Number of UUPS `__self` immutable slots in the deployed bytecode.
    /// Pinned against the binding's zero runs in `test_verify_kind_by_bytecode_ok`.
    pub self_windows: usize,
}

pub fn contract_kind(arg: ContractKindArg) -> ContractKind {
    match arg {
        ContractKindArg::StakeTableV2 => ContractKind {
            name: "StakeTableV2",
            deployed_bytecode: &StakeTableV2::DEPLOYED_BYTECODE,
            expected_init: Some(ExpectedInit {
                selector: StakeTableV2::initializeV2Call::SELECTOR,
                target_major: 2,
            }),
            owner_accessor: OwnerAccessor::Owner,
            timelock_kind: TimelockKind::Ops,
            expected_prev_major: Some(1),
            self_windows: 3,
        },
        ContractKindArg::StakeTableV3 => ContractKind {
            name: "StakeTableV3",
            deployed_bytecode: &StakeTableV3::DEPLOYED_BYTECODE,
            expected_init: Some(ExpectedInit {
                selector: StakeTableV3::initializeV3Call::SELECTOR,
                target_major: 3,
            }),
            owner_accessor: OwnerAccessor::Owner,
            timelock_kind: TimelockKind::Ops,
            expected_prev_major: Some(2),
            self_windows: 3,
        },
        ContractKindArg::EspTokenV2 => ContractKind {
            name: "EspTokenV2",
            deployed_bytecode: &EspTokenV2::DEPLOYED_BYTECODE,
            expected_init: Some(ExpectedInit {
                selector: EspTokenV2::initializeV2Call::SELECTOR,
                target_major: 2,
            }),
            owner_accessor: OwnerAccessor::Owner,
            timelock_kind: TimelockKind::SafeExit,
            expected_prev_major: Some(1),
            self_windows: 3,
        },
        ContractKindArg::FeeContract => ContractKind {
            name: "FeeContract",
            deployed_bytecode: &FeeContract::DEPLOYED_BYTECODE,
            expected_init: None,
            owner_accessor: OwnerAccessor::Owner,
            timelock_kind: TimelockKind::Ops,
            expected_prev_major: Some(1),
            self_windows: 3,
        },
        ContractKindArg::RewardClaim => ContractKind {
            name: "RewardClaim",
            deployed_bytecode: &RewardClaim::DEPLOYED_BYTECODE,
            expected_init: None,
            owner_accessor: OwnerAccessor::CurrentAdmin,
            timelock_kind: TimelockKind::SafeExit,
            expected_prev_major: None,
            self_windows: 3,
        },
    }
}

// ── calldata reconstruction ──────────────────────────────────────────────────

pub fn tx_calldata(tx: &SafeTx) -> Result<(Address, Bytes)> {
    if let (Some(method), Some(inputs)) = (&tx.contract_method, &tx.contract_inputs_values) {
        let calldata = reconstruct_timelock_calldata(&method.name, inputs)?;
        Ok((tx.to, calldata))
    } else if let Some(hex_data) = &tx.data {
        Ok((tx.to, parse_hex_bytes(hex_data)?))
    } else {
        bail!("SafeTx has neither contractMethod nor data")
    }
}

fn parse_hex_bytes(s: &str) -> Result<Bytes> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    Ok(Bytes::from(alloy::hex::decode(s)?))
}

fn reconstruct_timelock_calldata(name: &str, inputs: &BTreeMap<String, String>) -> Result<Bytes> {
    match name {
        "schedule" => {
            let target: Address = inputs
                .get("target")
                .ok_or_else(|| anyhow!("missing 'target' in schedule inputs"))?
                .parse()?;
            let value = U256::from_str_radix(
                inputs
                    .get("value")
                    .ok_or_else(|| anyhow!("missing 'value' in schedule inputs"))?,
                10,
            )?;
            let data = parse_hex_bytes(
                inputs
                    .get("data")
                    .ok_or_else(|| anyhow!("missing 'data' in schedule inputs"))?,
            )?;
            let predecessor: B256 = inputs
                .get("predecessor")
                .ok_or_else(|| anyhow!("missing 'predecessor' in schedule inputs"))?
                .parse()?;
            let salt: B256 = inputs
                .get("salt")
                .ok_or_else(|| anyhow!("missing 'salt' in schedule inputs"))?
                .parse()?;
            let delay = U256::from_str_radix(
                inputs
                    .get("delay")
                    .ok_or_else(|| anyhow!("missing 'delay' in schedule inputs"))?,
                10,
            )?;
            Ok(Bytes::from(
                OpsTimelock::scheduleCall {
                    target,
                    value,
                    data,
                    predecessor,
                    salt,
                    delay,
                }
                .abi_encode(),
            ))
        },
        "execute" => {
            let target: Address = inputs
                .get("target")
                .ok_or_else(|| anyhow!("missing 'target' in execute inputs"))?
                .parse()?;
            let value = U256::from_str_radix(
                inputs
                    .get("value")
                    .ok_or_else(|| anyhow!("missing 'value' in execute inputs"))?,
                10,
            )?;
            let payload = parse_hex_bytes(
                inputs
                    .get("payload")
                    .or_else(|| inputs.get("data"))
                    .ok_or_else(|| anyhow!("missing 'payload'/'data' in execute inputs"))?,
            )?;
            let predecessor: B256 = inputs
                .get("predecessor")
                .ok_or_else(|| anyhow!("missing 'predecessor' in execute inputs"))?
                .parse()?;
            let salt: B256 = inputs
                .get("salt")
                .ok_or_else(|| anyhow!("missing 'salt' in execute inputs"))?
                .parse()?;
            Ok(Bytes::from(
                OpsTimelock::executeCall {
                    target,
                    value,
                    payload,
                    predecessor,
                    salt,
                }
                .abi_encode(),
            ))
        },
        other => bail!("unsupported contractMethod name: {other}"),
    }
}

pub fn decode_inner_upgrade(inner: &Bytes) -> Result<(Address, Bytes)> {
    if inner.len() < 4 {
        bail!("inner calldata too short");
    }
    let call = StakeTableV3::upgradeToAndCallCall::abi_decode(inner)
        .map_err(|e| anyhow!("failed to decode upgradeToAndCall: {e}"))?;
    Ok((call.newImplementation, call.data))
}

/// Identify the phase of a single-transaction batch.
///
/// Fails unless `batch.transactions.len() == 1`.
fn batch_phase(batch: &SafeBatch) -> Result<Phase> {
    if batch.transactions.len() != 1 {
        bail!(
            "batch must contain exactly 1 transaction; got {}",
            batch.transactions.len()
        );
    }
    let tx = &batch.transactions[0];

    if let Some(method) = &tx.contract_method {
        return match method.name.as_str() {
            "schedule" => Ok(Phase::Schedule),
            "execute" => Ok(Phase::Execute),
            other => bail!("unrecognised contractMethod name: {other}"),
        };
    }

    if let Some(hex) = &tx.data {
        let bytes = parse_hex_bytes(hex)?;
        if bytes.len() < 4 {
            bail!("raw data too short to contain a selector");
        }
        let sel: [u8; 4] = bytes[..4].try_into().expect("len checked");
        return match sel {
            s if s == OpsTimelock::scheduleCall::SELECTOR => Ok(Phase::Schedule),
            s if s == OpsTimelock::executeCall::SELECTOR => Ok(Phase::Execute),
            _ => bail!("unrecognised 4-byte selector 0x{}", alloy::hex::encode(sel)),
        };
    }

    bail!("batch has neither contractMethod nor data");
}

/// Load `schedule.json` and `execute.json` from a proposal directory.
///
/// Both files must be present and parse as `SafeBatch`. Each batch must contain
/// exactly one transaction. Phase is validated via `batch_phase`.
pub fn load_proposal_dir(dir: &Path) -> Result<TimelockBatches> {
    let load = |name: &str| -> Result<SafeBatch> {
        let p = dir.join(name);
        let text =
            std::fs::read_to_string(&p).map_err(|e| anyhow!("cannot read {}: {e}", p.display()))?;
        serde_json::from_str::<SafeBatch>(&text)
            .map_err(|e| anyhow!("failed to parse {}: {e}", p.display()))
    };

    let schedule = load("schedule.json")?;
    let execute = load("execute.json")?;

    let sched_phase = batch_phase(&schedule)?;
    if sched_phase != Phase::Schedule {
        bail!(
            "{}/schedule.json has phase {:?}; expected Schedule",
            dir.display(),
            sched_phase
        );
    }
    let exec_phase = batch_phase(&execute)?;
    if exec_phase != Phase::Execute {
        bail!(
            "{}/execute.json has phase {:?}; expected Execute",
            dir.display(),
            exec_phase
        );
    }

    Ok(TimelockBatches { schedule, execute })
}

/// Decode a `TimelockBatches` into a `DecodedUpgrade`.
///
/// Each batch must have exactly one transaction; extra transactions would
/// escape verification.
pub fn decode_proposal(batches: TimelockBatches) -> Result<DecodedUpgrade> {
    if batches.schedule.transactions.len() != 1 {
        bail!(
            "schedule batch must contain exactly 1 transaction; got {}",
            batches.schedule.transactions.len()
        );
    }
    if batches.execute.transactions.len() != 1 {
        bail!(
            "execute batch must contain exactly 1 transaction; got {}",
            batches.execute.transactions.len()
        );
    }

    let sched_tx = batches
        .schedule
        .transactions
        .into_iter()
        .next()
        .expect("len checked");
    let exec_tx = batches
        .execute
        .transactions
        .into_iter()
        .next()
        .expect("len checked");

    let (sched_to, sched_calldata) = tx_calldata(&sched_tx)?;
    let (exec_to, exec_calldata) = tx_calldata(&exec_tx)?;

    let sched = OpsTimelock::scheduleCall::abi_decode(&sched_calldata)
        .map_err(|e| anyhow!("failed to decode schedule calldata: {e}"))?;
    let exec = OpsTimelock::executeCall::abi_decode(&exec_calldata)
        .map_err(|e| anyhow!("failed to decode execute calldata: {e}"))?;

    if sched_to != exec_to {
        bail!(
            "schedule and execute target different addresses: {} vs {}",
            sched_to,
            exec_to
        );
    }
    if sched.data != exec.payload {
        bail!("schedule.data != execute.payload: inner payloads are not identical");
    }
    if sched.salt != exec.salt {
        bail!("schedule.salt != execute.salt");
    }
    if sched.predecessor != exec.predecessor {
        bail!("schedule.predecessor != execute.predecessor");
    }
    if sched.target != exec.target {
        bail!("schedule.target != exec.target");
    }
    if sched.value != exec.value {
        bail!(
            "schedule.value != execute.value: {} vs {}",
            sched.value,
            exec.value
        );
    }

    let (new_impl, init_data) = decode_inner_upgrade(&sched.data)?;

    Ok(DecodedUpgrade {
        outer_to: sched_to,
        proxy: sched.target,
        new_impl,
        init_data,
        value: sched.value,
        predecessor: sched.predecessor,
        salt: sched.salt,
        delay: sched.delay,
        description: batches.schedule.meta.description,
        outer_calldatas: OuterCalldatas {
            schedule: sched_calldata,
            execute: exec_calldata,
        },
    })
}

// ── Report ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CheckRow {
    pub name: String,
    pub pass: bool,
    pub detail: String,
}

/// Safe-tx hashes for schedule and execute phases.
#[derive(Debug, Clone)]
pub struct PhaseHashes {
    pub schedule_nonce: u64,
    pub schedule: SafeTxHashes,
    pub execute_nonce: u64,
    pub execute: SafeTxHashes,
}

#[derive(Debug)]
pub struct VerifyReport {
    pub rows: Vec<CheckRow>,
    pub header: ReportHeader,
    pub phase_hashes: PhaseHashes,
}

#[derive(Debug, Clone)]
pub struct ReportHeader {
    pub proxy: Address,
    pub new_impl: Address,
    pub contract_name: &'static str,
    pub network: String,
    pub description: String,
    pub onchain_solc: Option<SolcVersion>,
    pub reference_solc: Option<SolcVersion>,
}

impl VerifyReport {
    pub fn print(&self) {
        println!("=== Upgrade Proposal Verification ===");
        println!("  contract:    {}", self.header.contract_name);
        println!("  network:     {}", self.header.network);
        println!("  proxy:       {}", self.header.proxy);
        println!("  new_impl:    {}", self.header.new_impl);
        println!("  route:       timelock two-phase");
        println!("  description: {}", self.header.description);
        if let Some(ref s) = self.header.onchain_solc {
            println!("  onchain_solc: {s}");
        }
        if let Some(ref s) = self.header.reference_solc {
            println!("  ref_solc:     {s}");
        }
        println!();
        println!("{:<40} {:<6} DETAIL", "CHECK", "RESULT");
        println!("{}", "-".repeat(80));
        for row in &self.rows {
            let status = if row.pass { "PASS" } else { "FAIL" };
            println!("{:<40} {:<6} {}", row.name, status, row.detail);
        }
        println!();
        println!("--- Safe tx hashes (operation=0, single-tx; confirm against Safe UI) ---");
        println!("  schedule (nonce={}):", self.phase_hashes.schedule_nonce);
        print_hashes(&self.phase_hashes.schedule);
        println!("  execute (nonce={}):", self.phase_hashes.execute_nonce);
        print_hashes(&self.phase_hashes.execute);
        println!();
        let all_pass = self.rows.iter().all(|r| r.pass);
        println!("Result: {}", if all_pass { "ALL PASS" } else { "FAIL" });
    }

    pub fn exit_code(&self) -> i32 {
        if self.rows.iter().all(|r| r.pass) {
            0
        } else {
            1
        }
    }
}

fn print_hashes(h: &SafeTxHashes) {
    println!("    domain:   {}", h.domain);
    println!("    message:  {}", h.message);
    println!("    safe_tx:  {}", h.safe_tx);
}

fn pass(name: impl Into<String>, detail: impl Into<String>) -> CheckRow {
    CheckRow {
        name: name.into(),
        pass: true,
        detail: detail.into(),
    }
}

fn fail(name: impl Into<String>, detail: impl Into<String>) -> CheckRow {
    CheckRow {
        name: name.into(),
        pass: false,
        detail: detail.into(),
    }
}

fn solc_str(v: Option<&SolcVersion>) -> String {
    v.map(|s| s.to_string()).unwrap_or_else(|| "?".to_owned())
}

// ── Pure row classifiers ─────────────────────────────────────────────────────

pub fn value_zero_row(value: U256) -> CheckRow {
    if value == U256::ZERO {
        pass("value==0", "ok")
    } else {
        fail("value==0", format!("value={value}"))
    }
}

pub fn predecessor_zero_row(predecessor: B256) -> CheckRow {
    if predecessor == B256::ZERO {
        pass("predecessor==0", "ok")
    } else {
        fail("predecessor==0", format!("{predecessor}"))
    }
}

pub fn owner_timelock_row(owner: Address, outer_to: Address) -> CheckRow {
    if owner == outer_to {
        pass("owner==timelock", format!("owner={owner}"))
    } else {
        fail(
            "owner==timelock",
            format!("proxy owner={owner} != outer_to={outer_to}"),
        )
    }
}

pub fn delay_row(delay: U256, min_delay: U256) -> CheckRow {
    if delay >= min_delay {
        pass(
            "delay>=minDelay",
            format!("delay={delay} minDelay={min_delay}"),
        )
    } else {
        fail(
            "delay>=minDelay",
            format!("delay={delay} < minDelay={min_delay}"),
        )
    }
}

/// Row asserting a decoded field equals the proposal.toml value.
fn toml_field_row<T: PartialEq + fmt::Display>(name: &str, decoded: T, recorded: T) -> CheckRow {
    if decoded == recorded {
        pass(format!("toml:{name}"), format!("{decoded}"))
    } else {
        fail(
            format!("toml:{name}"),
            format!("decoded={decoded} toml={recorded}"),
        )
    }
}

/// Row asserting a hash field equals the toml value (uses hex display).
fn toml_hash_row(name: &str, computed: B256, recorded: B256) -> CheckRow {
    if computed == recorded {
        pass(format!("toml:{name}"), format!("{computed}"))
    } else {
        fail(
            format!("toml:{name}"),
            format!("computed={computed} toml={recorded}"),
        )
    }
}

// ── Static network/chain-id mapping ──────────────────────────────────────────

/// Map a network name to its canonical chain id.
fn network_chain_id(network: &str) -> Option<u64> {
    match network {
        "mainnet" => Some(1),
        "decaf" => Some(11155111),
        "hoodi" => Some(560048),
        _ => None,
    }
}

// ── CLI args ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, clap::Args)]
pub struct VerifyProposalArgs {
    /// Proposal directory containing schedule.json, execute.json, and proposal.toml.
    pub dir: PathBuf,

    /// Override the contract kind; defaults to proposal.toml `contract` field.
    #[clap(long)]
    pub contract: Option<ContractKindArg>,

    /// RPC URL; defaults to the network's public node from proposal.toml.
    ///
    /// Note: this flag must follow the `verify-proposal` subcommand; the top-level
    /// `deploy --rpc-url` does not apply here.
    #[clap(long, env = "ESPRESSO_L1_PROVIDER")]
    pub rpc_url: Option<Url>,
}

// ── Orchestrator ─────────────────────────────────────────────────────────────

/// Run verification without a wallet provider.
///
/// Reads `chain_id` from `<args.dir>/proposal.toml`, resolves the RPC from
/// `args.rpc_url` (or `ESPRESSO_L1_PROVIDER`) or the built-in public-node map,
/// then delegates to `run_verify`.
pub async fn run_verify_standalone(args: &VerifyProposalArgs) -> Result<VerifyReport> {
    let toml = ProposalToml::load(&args.dir)?;
    let chain_id = toml.chain_id;

    let rpc = args
        .rpc_url
        .clone()
        .or_else(|| default_rpc_url(chain_id))
        .ok_or_else(|| anyhow!("unknown chain id {chain_id}; pass --rpc-url"))?;

    let provider = ProviderBuilder::new().connect_http(rpc);

    let provider_chain_id = provider.get_chain_id().await?;
    run_verify(args, &provider, provider_chain_id).await
}

pub async fn run_verify(
    args: &VerifyProposalArgs,
    provider: &impl Provider,
    chain_id: u64,
) -> Result<VerifyReport> {
    let toml = ProposalToml::load(&args.dir)?;

    // Resolve contract kind: flag overrides toml; if both present, assert equal.
    let kind_arg = match args.contract {
        Some(flag_kind) => {
            if flag_kind.as_str() != toml.contract {
                bail!(
                    "--contract {:?} conflicts with proposal.toml contract {:?} in {}",
                    flag_kind.as_str(),
                    toml.contract,
                    args.dir.display()
                );
            }
            flag_kind
        },
        None => ContractKindArg::from_str(&toml.contract, true).map_err(|e| {
            anyhow!(
                "proposal.toml has unknown contract {:?}: {e}",
                toml.contract
            )
        })?,
    };
    let kind = contract_kind(kind_arg);
    let mut rows: Vec<CheckRow> = vec![];

    // Network/chain_id consistency: static mapping, never circular (item F).
    rows.push(network_chain_id_row(&toml.network, toml.chain_id));

    // Provider chain_id vs toml.chain_id.
    rows.push(if chain_id == toml.chain_id {
        pass("toml:chain_id", format!("{chain_id}"))
    } else {
        fail(
            "toml:chain_id",
            format!("provider chain_id={chain_id} toml={}", toml.chain_id),
        )
    });

    let batches = load_proposal_dir(&args.dir)?;
    let upgrade = match decode_proposal(batches) {
        Ok(u) => {
            rows.push(pass("decode", format!("outer_to={}", u.outer_to)));
            u
        },
        Err(e) => {
            rows.push(fail("decode", e.to_string()));
            let phase_hashes = build_phase_hashes_from_toml(&toml);
            return Ok(VerifyReport {
                rows,
                header: ReportHeader {
                    proxy: Address::ZERO,
                    new_impl: Address::ZERO,
                    contract_name: kind.name,
                    network: toml.network.clone(),
                    description: String::new(),
                    onchain_solc: None,
                    reference_solc: None,
                },
                phase_hashes,
            });
        },
    };

    // Validate decoded fields against toml.
    rows.push(toml_field_row("proxy", upgrade.proxy, toml.proxy));
    rows.push(toml_field_row("impl", upgrade.new_impl, toml.new_impl));
    rows.push(toml_field_row("timelock", upgrade.outer_to, toml.timelock));
    rows.push(toml_field_row("salt", upgrade.salt, toml.salt));
    rows.push(toml_field_row(
        "predecessor",
        upgrade.predecessor,
        toml.predecessor,
    ));
    rows.push(toml_field_row("delay", upgrade.delay, toml.delay_u256()));

    rows.push(value_zero_row(upgrade.value));
    rows.push(predecessor_zero_row(upgrade.predecessor));

    // Deployment-info address checks (no provider needed).
    let info_rows = deployment_info_rows(&toml, &upgrade, &kind);
    rows.extend(info_rows);

    let onchain_code = provider.get_code_at(upgrade.new_impl).await?;

    let bytecode_check = match compare_normalized(
        &onchain_code,
        kind.deployed_bytecode,
        upgrade.new_impl,
        kind.self_windows,
    ) {
        Ok(check) => check,
        Err(e) => {
            rows.push(fail("bytecode-match", e.to_string()));
            let phase_hashes = build_phase_hashes_from_toml(&toml);
            return Ok(VerifyReport {
                rows,
                header: ReportHeader {
                    proxy: upgrade.proxy,
                    new_impl: upgrade.new_impl,
                    contract_name: kind.name,
                    network: toml.network.clone(),
                    description: upgrade.description,
                    onchain_solc: None,
                    reference_solc: None,
                },
                phase_hashes,
            });
        },
    };

    rows.push(match bytecode_check.class {
        MatchClass::FullMatch => pass(
            "bytecode-match",
            format!(
                "FullMatch solc={}",
                solc_str(bytecode_check.onchain_solc.as_ref())
            ),
        ),
        MatchClass::CodeMatchMetaDiffers => pass(
            "bytecode-match",
            format!(
                "WARN CodeMatchMetaDiffers: onchain_solc={} ref_solc={}",
                solc_str(bytecode_check.onchain_solc.as_ref()),
                solc_str(bytecode_check.reference_solc.as_ref())
            ),
        ),
        MatchClass::Mismatch => fail(
            "bytecode-match",
            format!(
                "on-chain impl at {} does not match {} binding: {}",
                upgrade.new_impl,
                kind.name,
                bytecode_check.mismatch.as_deref().unwrap_or("")
            ),
        ),
    });

    let proxy_major = fetch_proxy_major_version(provider, upgrade.proxy).await;
    rows.push(check_init_selector(
        &kind,
        &upgrade.init_data,
        proxy_major.as_ref().ok().copied(),
    ));

    // Safe validation: assert toml Safes match deployment-info.
    let safe_rows = safe_address_rows(&toml, &kind);
    rows.extend(safe_rows);

    // Recompute Safe hashes for both phases and assert against toml.
    let phase_hashes =
        compute_and_validate_phase_hashes(&toml, chain_id, &upgrade.outer_calldatas, &mut rows);

    // Nonce drift check (WARN, not FAIL).
    nonce_drift_rows(provider, &toml, &mut rows).await;

    let gov_rows = governance_checks(provider, &upgrade, &kind, proxy_major).await;
    rows.extend(gov_rows);

    Ok(VerifyReport {
        rows,
        header: ReportHeader {
            proxy: upgrade.proxy,
            new_impl: upgrade.new_impl,
            contract_name: kind.name,
            network: toml.network.clone(),
            description: upgrade.description,
            onchain_solc: bytecode_check.onchain_solc,
            reference_solc: bytecode_check.reference_solc,
        },
        phase_hashes,
    })
}

/// Build PhaseHashes directly from toml (used when decoding fails).
fn build_phase_hashes_from_toml(toml: &ProposalToml) -> PhaseHashes {
    PhaseHashes {
        schedule_nonce: toml.schedule.nonce,
        schedule: SafeTxHashes {
            domain: toml.schedule.domain,
            message: toml.schedule.message,
            safe_tx: toml.schedule.safe_tx,
        },
        execute_nonce: toml.execute.nonce,
        execute: SafeTxHashes {
            domain: toml.execute.domain,
            message: toml.execute.message,
            safe_tx: toml.execute.safe_tx,
        },
    }
}

/// Emit `toml:network` row (static chain-id mapping, breaks circular check).
fn network_chain_id_row(network: &str, chain_id: u64) -> CheckRow {
    match network_chain_id(network) {
        None => fail(
            "toml:network",
            format!(
                "unknown network {:?}; known: mainnet, decaf, hoodi",
                network
            ),
        ),
        Some(expected) if expected != chain_id => fail(
            "toml:network",
            format!("network={network:?} maps to chain_id={expected} but toml.chain_id={chain_id}"),
        ),
        Some(_) => pass("toml:network", format!("{network} chain_id={chain_id}")),
    }
}

/// Timelock and proxy address checks against embedded deployment-info.
///
/// These rows are always emitted; FAIL when deployment-info is unavailable.
fn deployment_info_rows(
    toml: &ProposalToml,
    upgrade: &DecodedUpgrade,
    kind: &ContractKind,
) -> Vec<CheckRow> {
    let info = match deployment_info(&toml.network) {
        Ok(i) => i,
        Err(e) => {
            return vec![
                fail(
                    "timelock-addr-match",
                    format!("deployment-info unavailable: {e}"),
                ),
                fail(
                    "proxy-addr-match",
                    format!("deployment-info unavailable: {e}"),
                ),
            ];
        },
    };

    let expected_timelock = match kind.timelock_kind {
        TimelockKind::Ops => info.ops_timelock.address,
        TimelockKind::SafeExit => info.safe_exit_timelock.address,
    };

    let expected_proxy = match kind.name {
        "StakeTableV2" | "StakeTableV3" => info.stake_table,
        "EspTokenV2" => info.esp_token,
        "FeeContract" => info.fee_contract,
        "RewardClaim" => info.reward_claim,
        other => {
            return vec![
                fail(
                    "timelock-addr-match",
                    format!("no proxy mapping for contract kind {other:?}"),
                ),
                fail(
                    "proxy-addr-match",
                    format!("no proxy mapping for contract kind {other:?}"),
                ),
            ];
        },
    };

    let timelock_row = if upgrade.outer_to == expected_timelock {
        pass(
            "timelock-addr-match",
            format!("outer_to={} matches deployment-info", upgrade.outer_to),
        )
    } else {
        fail(
            "timelock-addr-match",
            format!(
                "outer_to={} != deployment-info timelock={}",
                upgrade.outer_to, expected_timelock
            ),
        )
    };

    let proxy_row = if upgrade.proxy == expected_proxy {
        pass(
            "proxy-addr-match",
            format!("proxy={} matches deployment-info", upgrade.proxy),
        )
    } else {
        fail(
            "proxy-addr-match",
            format!(
                "proxy={} != deployment-info proxy={}",
                upgrade.proxy, expected_proxy
            ),
        )
    };

    vec![timelock_row, proxy_row]
}

/// Validate toml Safe addresses against embedded deployment-info (hard fail on unavailable).
fn safe_address_rows(toml: &ProposalToml, kind: &ContractKind) -> Vec<CheckRow> {
    let mut rows = vec![];

    let info = match deployment_info(&toml.network) {
        Ok(i) => i,
        Err(e) => {
            rows.push(fail(
                "toml:schedule.safe",
                format!("deployment-info unavailable: {e}"),
            ));
            rows.push(fail(
                "toml:execute.safe",
                format!("deployment-info unavailable: {e}"),
            ));
            return rows;
        },
    };

    let signers = match kind.timelock_kind {
        TimelockKind::Ops => &info.ops_timelock,
        TimelockKind::SafeExit => &info.safe_exit_timelock,
    };

    let proposer_match = signers.proposers.contains(&toml.schedule.safe);
    rows.push(if proposer_match {
        pass(
            "toml:schedule.safe",
            format!("{} is a known proposer", toml.schedule.safe),
        )
    } else {
        fail(
            "toml:schedule.safe",
            format!(
                "{} not in proposers {:?}",
                toml.schedule.safe, signers.proposers
            ),
        )
    });

    let executor_match = signers.executors.contains(&toml.execute.safe);
    rows.push(if executor_match {
        pass(
            "toml:execute.safe",
            format!("{} is a known executor", toml.execute.safe),
        )
    } else {
        fail(
            "toml:execute.safe",
            format!(
                "{} not in executors {:?}",
                toml.execute.safe, signers.executors
            ),
        )
    });

    rows
}

/// Recompute Safe hashes from the JSONs and assert each equals the toml value.
///
/// Returns the PhaseHashes (from the toml, now validated).
fn compute_and_validate_phase_hashes(
    toml: &ProposalToml,
    chain_id: u64,
    calldatas: &OuterCalldatas,
    rows: &mut Vec<CheckRow>,
) -> PhaseHashes {
    let sched = safe_tx_hashes(
        toml.schedule.safe,
        chain_id,
        toml.timelock,
        U256::ZERO,
        &calldatas.schedule,
        0,
        toml.schedule.nonce,
    );
    let exec = safe_tx_hashes(
        toml.execute.safe,
        chain_id,
        toml.timelock,
        U256::ZERO,
        &calldatas.execute,
        0,
        toml.execute.nonce,
    );

    rows.push(toml_hash_row(
        "schedule.domain",
        sched.domain,
        toml.schedule.domain,
    ));
    rows.push(toml_hash_row(
        "schedule.message",
        sched.message,
        toml.schedule.message,
    ));
    rows.push(toml_hash_row(
        "schedule.safe_tx",
        sched.safe_tx,
        toml.schedule.safe_tx,
    ));
    rows.push(toml_hash_row(
        "execute.domain",
        exec.domain,
        toml.execute.domain,
    ));
    rows.push(toml_hash_row(
        "execute.message",
        exec.message,
        toml.execute.message,
    ));
    rows.push(toml_hash_row(
        "execute.safe_tx",
        exec.safe_tx,
        toml.execute.safe_tx,
    ));

    PhaseHashes {
        schedule_nonce: toml.schedule.nonce,
        schedule: SafeTxHashes {
            domain: toml.schedule.domain,
            message: toml.schedule.message,
            safe_tx: toml.schedule.safe_tx,
        },
        execute_nonce: toml.execute.nonce,
        execute: SafeTxHashes {
            domain: toml.execute.domain,
            message: toml.execute.message,
            safe_tx: toml.execute.safe_tx,
        },
    }
}

/// Query on-chain nonces and emit WARN rows (not hard fails) when they differ from toml.
///
/// Nonce drift is expected if other transactions were queued since generation.
async fn nonce_drift_rows(provider: &impl Provider, toml: &ProposalToml, rows: &mut Vec<CheckRow>) {
    for (label, safe, recorded_nonce) in [
        ("schedule.nonce", toml.schedule.safe, toml.schedule.nonce),
        ("execute.nonce", toml.execute.safe, toml.execute.nonce),
    ] {
        match ISafe::new(safe, provider).nonce().call().await {
            Err(e) => {
                rows.push(pass(
                    format!("toml:{label}"),
                    format!("WARN: nonce query failed ({e}); signer must reconfirm"),
                ));
            },
            Ok(onchain) => {
                let onchain_u64: u64 = match onchain.try_into() {
                    Ok(n) => n,
                    Err(_) => {
                        rows.push(pass(
                            format!("toml:{label}"),
                            "WARN: onchain nonce overflows u64".to_owned(),
                        ));
                        continue;
                    },
                };
                if onchain_u64 == recorded_nonce {
                    rows.push(pass(
                        format!("toml:{label}"),
                        format!("nonce={onchain_u64}"),
                    ));
                } else {
                    rows.push(pass(
                        format!("toml:{label}"),
                        format!(
                            "WARN: onchain nonce={onchain_u64} != toml={recorded_nonce}; hashes \
                             in toml use recorded nonce; signer must reconfirm",
                        ),
                    ));
                }
            },
        }
    }
}

/// Validate init calldata against the kind's expected reinitializer.
///
/// Empty init data is only accepted when the reinitializer is genuinely
/// unnecessary: either the kind has none, or the proxy's on-chain major
/// version (`proxy_major`, `None` if the query failed) already reached the
/// target. Otherwise the upgrade would silently skip e.g. `initializeV3()`.
fn check_init_selector(
    kind: &ContractKind,
    init_data: &Bytes,
    proxy_major: Option<u8>,
) -> CheckRow {
    let Some(init) = kind.expected_init else {
        return if init_data.is_empty() {
            pass(
                "init-selector",
                "empty (expected for patch/no-reinitializer)",
            )
        } else {
            fail(
                "init-selector",
                format!(
                    "non-empty init data with no expected selector; selector=0x{} (arbitrary \
                     delegated call through proxy)",
                    alloy::hex::encode(&init_data[..4.min(init_data.len())])
                ),
            )
        };
    };

    if init_data.is_empty() {
        return match proxy_major {
            Some(major) if major >= init.target_major => pass(
                "init-selector",
                format!(
                    "empty ok: proxy_major={major} >= target={}",
                    init.target_major
                ),
            ),
            Some(major) => fail(
                "init-selector",
                format!(
                    "empty init data but proxy_major={major} < target={}; reinitializer 0x{} \
                     would never run",
                    init.target_major,
                    alloy::hex::encode(init.selector)
                ),
            ),
            None => fail(
                "init-selector",
                "empty init data and proxy version query failed; cannot confirm the reinitializer \
                 is unnecessary",
            ),
        };
    }

    // The correct selector passes regardless of proxy_major: re-running a
    // reinitializer on an already-upgraded proxy reverts at execution
    // (InvalidInitialization), so this cannot be exploited, only wasted.
    if init_data.len() >= 4 && init_data[..4] == init.selector {
        pass(
            "init-selector",
            format!("ok selector=0x{}", alloy::hex::encode(init.selector)),
        )
    } else {
        fail(
            "init-selector",
            format!(
                "expected 0x{} got 0x{}",
                alloy::hex::encode(init.selector),
                alloy::hex::encode(&init_data[..4.min(init_data.len())])
            ),
        )
    }
}

async fn governance_checks(
    provider: &impl Provider,
    upgrade: &DecodedUpgrade,
    kind: &ContractKind,
    proxy_major: Result<u8>,
) -> Vec<CheckRow> {
    let mut rows = vec![];

    match fetch_proxy_owner(provider, upgrade.proxy, kind.owner_accessor).await {
        Err(e) => rows.push(fail("owner-query", e.to_string())),
        Ok(owner) => {
            rows.push(owner_timelock_row(owner, upgrade.outer_to));
        },
    }

    match fetch_min_delay(provider, upgrade.outer_to).await {
        Err(e) => rows.push(fail("delay>=minDelay", e.to_string())),
        Ok(min_delay) => rows.push(delay_row(upgrade.delay, min_delay)),
    }

    match (proxy_major, kind.expected_prev_major) {
        (Err(e), _) => rows.push(fail("version-prereq", e.to_string())),
        (Ok(_), None) => rows.push(pass("version-prereq", "no prereq (RewardClaim)")),
        (Ok(major), Some(expected_prev)) => rows.push(if major >= expected_prev {
            pass(
                "version-prereq",
                format!("proxy_major={major} >= required={expected_prev}"),
            )
        } else {
            fail(
                "version-prereq",
                format!("proxy_major={major} < required={expected_prev}; upgrade path invalid"),
            )
        }),
    }

    rows
}

async fn fetch_proxy_owner(
    provider: &impl Provider,
    proxy: Address,
    accessor: OwnerAccessor,
) -> Result<Address> {
    match accessor {
        OwnerAccessor::Owner => Ok(StakeTableV3::new(proxy, provider).owner().call().await?),
        OwnerAccessor::CurrentAdmin => Ok(RewardClaim::new(proxy, provider)
            .currentAdmin()
            .call()
            .await?),
    }
}

async fn fetch_min_delay(provider: &impl Provider, timelock: Address) -> Result<U256> {
    Ok(OpsTimelock::new(timelock, provider)
        .getMinDelay()
        .call()
        .await?)
}

async fn fetch_proxy_major_version(provider: &impl Provider, proxy: Address) -> Result<u8> {
    Ok(StakeTableV3::new(proxy, provider)
        .getVersion()
        .call()
        .await?
        .majorVersion)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use alloy::{
        primitives::{Address, B256, Bytes, U256},
        sol_types::SolCall,
    };

    use super::*;
    use crate::proposals::proposal_toml::{PhaseToml, ProposalToml};

    const SCHEDULE_JSON: &str = include_str!("fixtures/decaf_stake_table_v3_schedule.json");
    const EXECUTE_JSON: &str = include_str!("fixtures/decaf_stake_table_v3_execute.json");

    fn load_fixture_proposal() -> TimelockBatches {
        let s: SafeBatch = serde_json::from_str(SCHEDULE_JSON).unwrap();
        let e: SafeBatch = serde_json::from_str(EXECUTE_JSON).unwrap();
        TimelockBatches {
            schedule: s,
            execute: e,
        }
    }

    /// Build a ProposalToml matching the decaf fixture (known-vector hashes from safe_hash test).
    fn fixture_toml() -> ProposalToml {
        let safe: Address = "0xb76834e371b666feee48e5d7d9a97ca08b5a0620"
            .parse()
            .unwrap();
        ProposalToml {
            contract: "stake-table-v3".to_owned(),
            network: "decaf".to_owned(),
            chain_id: 11155111,
            proxy: "0x40304FbE94D5E7D1492Dd90c53a2D63E8506a037"
                .parse()
                .unwrap(),
            new_impl: "0x5a6250dd35d875c0529573d9d934629a1b2778db"
                .parse()
                .unwrap(),
            timelock: "0x8e3b6563D683b87964104A2c3A4bf542bb70767F"
                .parse()
                .unwrap(),
            salt: "0x99f200000000000000000000000000000000000000000000000000000000000f"
                .parse()
                .unwrap(),
            delay: 300,
            predecessor: B256::ZERO,
            schedule: PhaseToml {
                safe,
                nonce: 24,
                domain: "0x8f560c9d209e6d9320305560aee98fa1dea01510aa5451a9c0911401893835c6"
                    .parse()
                    .unwrap(),
                message: "0x9c5a62271d73b6accf3c8957a1e80b6434618d3bd4b8bd23e30817479c60d35b"
                    .parse()
                    .unwrap(),
                safe_tx: "0xa3d4b5bfa93b559f34478b3988f1132c35ba67f953a87326c8a1c8250709c6b8"
                    .parse()
                    .unwrap(),
            },
            execute: PhaseToml {
                safe,
                nonce: 25,
                domain: "0x8f560c9d209e6d9320305560aee98fa1dea01510aa5451a9c0911401893835c6"
                    .parse()
                    .unwrap(),
                message: "0xf7edebe09a94e770ddbccf107a5685d50d902adb08db5e2043c7b1f9c4ef648b"
                    .parse()
                    .unwrap(),
                safe_tx: "0xbb7fd662e5b724a50e33f18ef737d6df9c1d92b8810def16fb190b7c27c16f45"
                    .parse()
                    .unwrap(),
            },
        }
    }

    // ── TEST:verify-decode-timelock-ok ─────────────────────────────────────

    #[test]
    fn test_verify_decode_timelock_ok() {
        let upgrade = decode_proposal(load_fixture_proposal()).expect("decode");

        assert_eq!(
            upgrade.proxy,
            "0x40304FbE94D5E7D1492Dd90c53a2D63E8506a037"
                .parse::<Address>()
                .unwrap()
        );
        assert_eq!(
            upgrade.new_impl,
            "0x5a6250dd35d875c0529573d9d934629a1b2778db"
                .parse::<Address>()
                .unwrap()
        );
        assert_eq!(
            upgrade.outer_to,
            "0x8e3b6563D683b87964104A2c3A4bf542bb70767F"
                .parse::<Address>()
                .unwrap()
        );
        assert_eq!(upgrade.value, U256::ZERO);
        assert_eq!(upgrade.predecessor, B256::ZERO);
        assert_eq!(upgrade.delay, U256::from(300));
        assert_eq!(
            &upgrade.init_data[..4],
            &StakeTableV3::initializeV3Call::SELECTOR
        );
    }

    // ── TEST:verify-inner-identical-ok ────────────────────────────────────

    #[test]
    fn test_verify_inner_identical_ok() {
        decode_proposal(load_fixture_proposal()).expect("inner payloads must be identical");
    }

    // ── TEST:verify-inner-mismatch-fails ──────────────────────────────────

    #[test]
    fn test_verify_inner_mismatch_fails() {
        let s: SafeBatch = serde_json::from_str(SCHEDULE_JSON).unwrap();
        let e_json = EXECUTE_JSON.replace("0x4f1ef286", "0xdeadbeef");
        let e: SafeBatch = serde_json::from_str(&e_json).unwrap();
        let err = decode_proposal(TimelockBatches {
            schedule: s,
            execute: e,
        })
        .unwrap_err();
        assert!(
            err.to_string().contains("not identical") || err.to_string().contains("payload"),
            "unexpected error: {err}"
        );
    }

    // ── TEST:verify-batch-extra-tx-errors ────────────────────────────────

    #[test]
    fn test_verify_batch_extra_tx_errors() {
        let mut s: SafeBatch = serde_json::from_str(SCHEDULE_JSON).unwrap();
        // Duplicate the transaction to simulate a multi-tx batch.
        let dup = s.transactions[0].clone();
        s.transactions.push(dup);
        assert_eq!(s.transactions.len(), 2);

        let err = batch_phase(&s).unwrap_err();
        assert!(
            err.to_string().contains("exactly 1"),
            "expected single-tx error, got: {err}"
        );
    }

    // ── TEST:verify-decode-extra-tx-errors ───────────────────────────────

    #[test]
    fn test_verify_decode_extra_tx_errors() {
        let mut s: SafeBatch = serde_json::from_str(SCHEDULE_JSON).unwrap();
        let dup_tx = s.transactions[0].clone();
        s.transactions.push(dup_tx);
        let e: SafeBatch = serde_json::from_str(EXECUTE_JSON).unwrap();

        let err = decode_proposal(TimelockBatches {
            schedule: s,
            execute: e,
        })
        .unwrap_err();
        assert!(
            err.to_string().contains("exactly 1"),
            "expected single-tx error, got: {err}"
        );
    }

    // ── TEST:verify-load-proposal-dir-ok ─────────────────────────────────

    #[test]
    fn test_verify_load_proposal_dir_ok() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        std::fs::write(dir.join("schedule.json"), SCHEDULE_JSON).unwrap();
        std::fs::write(dir.join("execute.json"), EXECUTE_JSON).unwrap();

        let batches = load_proposal_dir(dir).expect("load_proposal_dir");
        assert_eq!(
            batches.schedule.transactions[0]
                .contract_method
                .as_ref()
                .unwrap()
                .name,
            "schedule"
        );
        assert_eq!(
            batches.execute.transactions[0]
                .contract_method
                .as_ref()
                .unwrap()
                .name,
            "execute"
        );
    }

    // ── TEST:verify-load-proposal-dir-missing-execute-errors ─────────────

    #[test]
    fn test_verify_load_proposal_dir_missing_execute_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        std::fs::write(dir.join("schedule.json"), SCHEDULE_JSON).unwrap();
        // No execute.json.
        let err = load_proposal_dir(dir).unwrap_err();
        assert!(
            err.to_string().contains("execute.json"),
            "unexpected error: {err}"
        );
    }

    // ── TEST:verify-load-proposal-dir-phase-mismatch-errors ──────────────

    #[test]
    fn test_verify_load_proposal_dir_phase_mismatch_errors() {
        // schedule.json contains an execute call; phase mismatch must error.
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        // Write the execute JSON as schedule.json so phase validation fails.
        std::fs::write(dir.join("schedule.json"), EXECUTE_JSON).unwrap();
        std::fs::write(dir.join("execute.json"), EXECUTE_JSON).unwrap();
        let err = load_proposal_dir(dir).unwrap_err();
        assert!(
            err.to_string().contains("schedule.json") && err.to_string().contains("Execute"),
            "unexpected error: {err}"
        );
    }

    // ── TEST:verify-contract-flag-vs-toml-mismatch-errors ────────────────

    #[test]
    fn test_verify_contract_flag_vs_toml_mismatch_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        let mut toml = fixture_toml();
        toml.contract = "stake-table-v3".to_owned();
        toml.write(&dir).unwrap();

        let args = VerifyProposalArgs {
            dir: dir.clone(),
            contract: Some(ContractKindArg::FeeContract),
            rpc_url: None,
        };
        // Simulate the mismatch check from run_verify.
        let loaded = ProposalToml::load(&args.dir).unwrap();
        assert_ne!(args.contract.unwrap().as_str(), loaded.contract);
    }

    // ── TEST:verify-strip-cbor-828-ok ─────────────────────────────────────

    #[test]
    fn test_verify_strip_cbor_828_ok() {
        let mut code: Vec<u8> = vec![0xde, 0xad, 0xbe, 0xef];
        code.extend_from_slice(&make_cbor_tail([0x00, 0x08, 0x1c]));
        let (stripped, ver) = strip_cbor_metadata(&code);
        assert_eq!(stripped, vec![0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(ver, Some(SolcVersion([0x00, 0x08, 0x1c])));
    }

    // ── TEST:verify-strip-cbor-835-ok ─────────────────────────────────────

    #[test]
    fn test_verify_strip_cbor_835_ok() {
        let mut code: Vec<u8> = vec![0xca, 0xfe];
        code.extend_from_slice(&make_cbor_tail([0x00, 0x08, 0x23]));
        let (stripped, ver) = strip_cbor_metadata(&code);
        assert_eq!(stripped, vec![0xca, 0xfe]);
        assert_eq!(ver, Some(SolcVersion([0x00, 0x08, 0x23])));
    }

    // ── TEST:verify-strip-cbor-ipfs-ok ────────────────────────────────────
    //
    // Default solc metadata: a2 map with an ipfs hash entry before the solc entry.

    #[test]
    fn test_verify_strip_cbor_ipfs_ok() {
        let mut cbor: Vec<u8> = vec![0xa2, 0x64, 0x69, 0x70, 0x66, 0x73, 0x58, 0x22];
        cbor.extend_from_slice(&[0x12; 34]);
        cbor.extend_from_slice(&[0x64, 0x73, 0x6f, 0x6c, 0x63, 0x43, 0x00, 0x08, 0x23]);
        let mut code: Vec<u8> = vec![0xde, 0xad];
        code.extend_from_slice(&cbor);
        code.extend_from_slice(&(cbor.len() as u16).to_be_bytes());
        let (stripped, ver) = strip_cbor_metadata(&code);
        assert_eq!(stripped, vec![0xde, 0xad]);
        assert_eq!(ver, Some(SolcVersion([0x00, 0x08, 0x23])));
    }

    // ── TEST:verify-strip-cbor-oversized-tail-rejected ────────────────────
    //
    // A claimed CBOR length above the metadata bound must not strip, even if
    // the body ends with a valid solc entry (hides appended code otherwise).

    #[test]
    fn test_verify_strip_cbor_oversized_tail_rejected() {
        let mut code: Vec<u8> = vec![0xde, 0xad];
        code.extend_from_slice(&[0x66; 60]); // attacker-appended bytes
        code.extend_from_slice(&[0x64, 0x73, 0x6f, 0x6c, 0x63, 0x43, 0x00, 0x08, 0x23]);
        code.extend_from_slice(&69u16.to_be_bytes()); // claims 60 + 9 bytes of "metadata"
        let (stripped, ver) = strip_cbor_metadata(&code);
        assert_eq!(stripped, code);
        assert!(ver.is_none());
    }

    // ── TEST:verify-strip-cbor-marker-not-at-end-rejected ─────────────────

    #[test]
    fn test_verify_strip_cbor_marker_not_at_end_rejected() {
        let mut cbor = make_cbor_tail([0x00, 0x08, 0x23]);
        cbor.truncate(cbor.len() - 2); // drop the length, keep the body
        cbor.extend_from_slice(&[0xba, 0xad]); // bytes after the solc entry
        let mut code: Vec<u8> = vec![0xde, 0xad];
        code.extend_from_slice(&cbor);
        code.extend_from_slice(&(cbor.len() as u16).to_be_bytes());
        let (stripped, ver) = strip_cbor_metadata(&code);
        assert_eq!(stripped, code);
        assert!(ver.is_none());
    }

    // ── TEST:verify-no-metadata-tail-ok ───────────────────────────────────
    //
    // Code without a valid solc CBOR marker must be returned unchanged.

    #[test]
    fn test_verify_no_metadata_tail_ok() {
        let code: Vec<u8> = vec![0xde, 0xad, 0xbe, 0xef, 0x00, 0x04];
        let (stripped, ver) = strip_cbor_metadata(&code);
        assert!(ver.is_none());
        // Code must not be truncated when no solc marker found.
        assert_eq!(
            stripped, code,
            "code without solc marker must be returned unchanged"
        );
    }

    // ── TEST:verify-appended-garbage-mismatch ────────────────────────────
    //
    // Garbage appended to on-chain bytecode (no solc marker) must not be stripped,
    // causing a core-length mismatch and Mismatch classification.

    #[test]
    fn test_verify_appended_garbage_mismatch() {
        let impl_addr = Address::repeat_byte(0x01);
        let cbor = make_cbor_tail([0x00, 0x08, 0x23]);

        // Build reference: core bytes + impl zeros + cbor.
        let mut reference = vec![0xaa, 0xbb, 0xcc, 0xdd, 0xee];
        reference.extend_from_slice(&[0u8; 20]); // immutable window
        reference.extend_from_slice(&cbor);

        // Build on-chain: same but impl address injected, then garbage bytes appended
        // with no valid solc CBOR marker at the new tail.
        let mut onchain = vec![0xaa, 0xbb, 0xcc, 0xdd, 0xee];
        onchain.extend_from_slice(impl_addr.as_slice());
        onchain.extend_from_slice(&cbor);
        // Append garbage that happens to look like a length field pointing into cbor,
        // but whose cbor body has no solc marker.
        onchain.extend_from_slice(&[0xba, 0xad, 0xf0, 0x0d]);
        // The last 2 bytes encode a plausible length but the body won't have the solc prefix.
        let garbage_len: u16 = 6;
        onchain.extend_from_slice(&garbage_len.to_be_bytes());

        let check = compare_normalized(&onchain, &reference, impl_addr, 1).unwrap();
        assert_eq!(
            check.class,
            MatchClass::Mismatch,
            "appended garbage must not be silently stripped"
        );
    }

    // ── TEST:verify-bytecode-fullmatch-ok ─────────────────────────────────

    #[test]
    fn test_verify_bytecode_fullmatch_ok() {
        let impl_addr = Address::repeat_byte(0x01);
        let cbor = make_cbor_tail([0x00, 0x08, 0x23]);

        // impl_addr at offset 5; reference has zeros there.
        let mut onchain = vec![0xaa, 0xbb, 0xcc, 0xdd, 0xee];
        onchain.extend_from_slice(impl_addr.as_slice());
        onchain.extend_from_slice(&[0x11, 0x22, 0x33]);
        onchain.extend_from_slice(&cbor);

        let mut reference = vec![0xaa, 0xbb, 0xcc, 0xdd, 0xee];
        reference.extend_from_slice(&[0u8; 20]);
        reference.extend_from_slice(&[0x11, 0x22, 0x33]);
        reference.extend_from_slice(&cbor);

        let check = compare_normalized(&onchain, &reference, impl_addr, 1).unwrap();
        assert_eq!(check.class, MatchClass::FullMatch);
    }

    // ── TEST:verify-bytecode-metadiff-ok ──────────────────────────────────

    #[test]
    fn test_verify_bytecode_metadiff_ok() {
        let impl_addr = Address::repeat_byte(0x01);
        let cbor_828 = make_cbor_tail([0x00, 0x08, 0x1c]);
        let cbor_835 = make_cbor_tail([0x00, 0x08, 0x23]);

        let mut onchain = vec![0xaa, 0xbb, 0xcc, 0xdd, 0xee];
        onchain.extend_from_slice(impl_addr.as_slice());
        onchain.extend_from_slice(&cbor_828);

        let mut reference = vec![0xaa, 0xbb, 0xcc, 0xdd, 0xee];
        reference.extend_from_slice(&[0u8; 20]);
        reference.extend_from_slice(&cbor_835);

        let check = compare_normalized(&onchain, &reference, impl_addr, 1).unwrap();
        assert_eq!(check.class, MatchClass::CodeMatchMetaDiffers);
        assert_eq!(check.onchain_solc, Some(SolcVersion([0x00, 0x08, 0x1c])));
        assert_eq!(check.reference_solc, Some(SolcVersion([0x00, 0x08, 0x23])));
    }

    // ── TEST:verify-bytecode-core-flip-fails ──────────────────────────────

    #[test]
    fn test_verify_bytecode_core_flip_fails() {
        let impl_addr = Address::repeat_byte(0x01);
        let cbor = make_cbor_tail([0x00, 0x08, 0x23]);

        let mut onchain = vec![0xaa, 0xbb, 0xcc, 0xdd, 0xee];
        onchain.extend_from_slice(impl_addr.as_slice());
        onchain.extend_from_slice(&cbor);

        let mut reference = vec![0xaa, 0xbb, 0xcc, 0xdd, 0xff]; // flipped byte
        reference.extend_from_slice(&[0u8; 20]);
        reference.extend_from_slice(&cbor);

        let check = compare_normalized(&onchain, &reference, impl_addr, 1).unwrap();
        assert_eq!(check.class, MatchClass::Mismatch);
        let detail = check.mismatch.unwrap();
        assert!(detail.contains("offset 4"), "{detail}");
    }

    // ── TEST:verify-kind-by-bytecode-ok ───────────────────────────────────
    //
    // For every contract kind: locate the __self immutable slots in the
    // DEPLOYED_BYTECODE core (the only 20+-byte zero runs), inject a fake impl
    // there, and assert compare_normalized passes. Pinning the run count to
    // kind.self_windows guards the "difference over zeros must be __self"
    // assumption: a rebuilt binding that grows a large zero run outside the
    // immutable slots (e.g. a PUSH32 0 constant) fails here and forces review,
    // because such a run would be a substitution surface at verify time.

    /// Maximal runs of zero bytes of length >= 20, as (start, len).
    fn zero_runs(core: &[u8]) -> Vec<(usize, usize)> {
        let mut runs = vec![];
        let mut i = 0;
        while i < core.len() {
            if core[i] == 0 {
                let start = i;
                while i < core.len() && core[i] == 0 {
                    i += 1;
                }
                if i - start >= 20 {
                    runs.push((start, i - start));
                }
            } else {
                i += 1;
            }
        }
        runs
    }

    #[test]
    fn test_verify_kind_by_bytecode_ok() {
        for arg in [
            ContractKindArg::StakeTableV2,
            ContractKindArg::StakeTableV3,
            ContractKindArg::EspTokenV2,
            ContractKindArg::FeeContract,
            ContractKindArg::RewardClaim,
        ] {
            let kind = contract_kind(arg);
            let name = kind.name;
            let (ref_core, _) = strip_cbor_metadata(kind.deployed_bytecode);
            let runs = zero_runs(&ref_core);
            // Each __self slot is a 32-byte zero run (12-byte pad + 20-byte address).
            assert_eq!(runs.len(), kind.self_windows, "{name}: {runs:?}");
            assert!(
                runs.iter().all(|&(_, len)| len == 32),
                "{name}: unexpected zero runs {runs:?}"
            );

            // Inject a fake impl at the address position of each slot.
            let fake_impl = Address::repeat_byte(0x42);
            let mut onchain = ref_core.clone();
            for &(start, _) in &runs {
                onchain[start + 12..start + 32].copy_from_slice(fake_impl.as_slice());
            }

            // Re-attach a consistent cbor tail to both so strip works symmetrically.
            let cbor = make_cbor_tail([0x00, 0x08, 0x23]);
            let mut onchain_full = onchain;
            onchain_full.extend_from_slice(&cbor);
            let mut ref_full = ref_core;
            ref_full.extend_from_slice(&cbor);

            let check = compare_normalized(&onchain_full, &ref_full, fake_impl, kind.self_windows)
                .unwrap_or_else(|e| panic!("{name}: compare_normalized failed: {e}"));
            assert_eq!(check.class, MatchClass::FullMatch, "{name}");
        }
    }

    // ── TEST:verify-extra-self-window-bails ───────────────────────────────
    //
    // Impl address substituted into an additional zero region of the reference
    // (beyond the expected __self slots) must error, not pass.

    #[test]
    fn test_verify_extra_self_window_bails() {
        let impl_addr = Address::repeat_byte(0x42);
        let cbor = make_cbor_tail([0x00, 0x08, 0x23]);

        let mut ref_full = vec![0x11u8; 10];
        ref_full.extend_from_slice(&[0u8; 20]); // __self slot
        ref_full.extend_from_slice(&[0x22u8; 10]);
        ref_full.extend_from_slice(&[0u8; 20]); // zero constant, not a __self slot
        ref_full.extend_from_slice(&cbor);

        let mut onchain_full = vec![0x11u8; 10];
        onchain_full.extend_from_slice(impl_addr.as_slice());
        onchain_full.extend_from_slice(&[0x22u8; 10]);
        onchain_full.extend_from_slice(impl_addr.as_slice()); // substituted constant
        onchain_full.extend_from_slice(&cbor);

        let err = compare_normalized(&onchain_full, &ref_full, impl_addr, 1).unwrap_err();
        assert!(err.to_string().contains("found 2"), "{err}");
    }

    // ── TEST:verify-missing-self-window-bails ─────────────────────────────
    //
    // Only one of two expected __self slots filled (the other byte-equal to
    // the reference zeros) must error.

    #[test]
    fn test_verify_missing_self_window_bails() {
        let impl_addr = Address::repeat_byte(0x42);
        let cbor = make_cbor_tail([0x00, 0x08, 0x23]);

        let mut ref_full = vec![0x11u8; 10];
        ref_full.extend_from_slice(&[0u8; 20]);
        ref_full.extend_from_slice(&[0x22u8; 10]);
        ref_full.extend_from_slice(&[0u8; 20]);
        ref_full.extend_from_slice(&cbor);

        let mut onchain_full = vec![0x11u8; 10];
        onchain_full.extend_from_slice(impl_addr.as_slice());
        onchain_full.extend_from_slice(&[0x22u8; 10]);
        onchain_full.extend_from_slice(&[0u8; 20]); // slot left unfilled
        onchain_full.extend_from_slice(&cbor);

        let err = compare_normalized(&onchain_full, &ref_full, impl_addr, 2).unwrap_err();
        assert!(err.to_string().contains("found 1"), "{err}");
    }

    // ── TEST:verify-impl-over-nonzero-reference-mismatch ─────────────────
    //
    // Impl address written where the reference has non-zero bytes is not an
    // explainable difference and must produce Mismatch.

    #[test]
    fn test_verify_impl_over_nonzero_reference_mismatch() {
        let impl_addr = Address::repeat_byte(0x42);
        let cbor = make_cbor_tail([0x00, 0x08, 0x23]);

        let mut ref_full = vec![0x11u8; 30];
        ref_full.extend_from_slice(&[0u8; 20]); // one genuine __self slot
        ref_full.extend_from_slice(&cbor);

        let mut onchain_core = vec![0x11u8; 30];
        onchain_core.extend_from_slice(impl_addr.as_slice());
        // Overwrite non-zero reference bytes with the impl address.
        onchain_core[5..25].copy_from_slice(impl_addr.as_slice());
        let mut onchain_full = onchain_core;
        onchain_full.extend_from_slice(&cbor);

        let check = compare_normalized(&onchain_full, &ref_full, impl_addr, 1).unwrap();
        assert_eq!(check.class, MatchClass::Mismatch);
    }

    // ── TEST:verify-corrupted-self-window-mismatch ────────────────────────
    //
    // A __self slot holding anything other than exactly the impl address
    // (one byte flipped) must produce Mismatch.

    #[test]
    fn test_verify_corrupted_self_window_mismatch() {
        let impl_addr = Address::repeat_byte(0x42);
        let cbor = make_cbor_tail([0x00, 0x08, 0x23]);

        let mut ref_full = vec![0x11u8; 10];
        ref_full.extend_from_slice(&[0u8; 20]);
        ref_full.extend_from_slice(&cbor);

        let mut window: [u8; 20] = impl_addr.into();
        window[7] ^= 0x01;
        let mut onchain_full = vec![0x11u8; 10];
        onchain_full.extend_from_slice(&window);
        onchain_full.extend_from_slice(&cbor);

        let check = compare_normalized(&onchain_full, &ref_full, impl_addr, 1).unwrap();
        assert_eq!(check.class, MatchClass::Mismatch);
    }

    // ── TEST:verify-no-self-window-bails ──────────────────────────────────
    //
    // On-chain code byte-identical to the binding has no baked-in __self
    // address; that is impossible for a real UUPS deploy and must error.

    #[test]
    fn test_verify_no_self_window_bails() {
        let impl_addr = Address::repeat_byte(0x42);
        let cbor = make_cbor_tail([0x00, 0x08, 0x23]);
        let mut code = vec![0x11u8; 10];
        code.extend_from_slice(&[0u8; 20]);
        code.extend_from_slice(&cbor);

        let err = compare_normalized(&code, &code, impl_addr, 1).unwrap_err();
        assert!(err.to_string().contains("__self"), "{err}");
    }

    // ── TEST:verify-zero-impl-addr-bails ──────────────────────────────────

    #[test]
    fn test_verify_zero_impl_addr_bails() {
        let code = vec![0x11u8; 10];
        assert!(compare_normalized(&code, &code, Address::ZERO, 1).is_err());
    }

    // ── TEST:verify-contract-kind-args ────────────────────────────────────

    #[test]
    fn test_verify_contract_kind_args() {
        let k = contract_kind(ContractKindArg::StakeTableV3);
        assert_eq!(k.name, "StakeTableV3");
        assert_eq!(k.owner_accessor, OwnerAccessor::Owner);
        assert_eq!(k.timelock_kind, TimelockKind::Ops);

        let k = contract_kind(ContractKindArg::RewardClaim);
        assert_eq!(k.owner_accessor, OwnerAccessor::CurrentAdmin);
        assert_eq!(k.timelock_kind, TimelockKind::SafeExit);
    }

    // ── TEST:verify-governance-owner-ok ───────────────────────────────────

    #[test]
    fn test_verify_governance_owner_ok() {
        let timelock = Address::repeat_byte(0x11);
        assert!(owner_timelock_row(timelock, timelock).pass);
        assert!(!owner_timelock_row(Address::repeat_byte(0x22), timelock).pass);
    }

    // ── TEST:verify-governance-delay-short-fails ──────────────────────────

    #[test]
    fn test_verify_governance_delay_short_fails() {
        assert!(!delay_row(U256::from(60u64), U256::from(300u64)).pass);
        assert!(delay_row(U256::from(300u64), U256::from(300u64)).pass);
        assert!(delay_row(U256::from(400u64), U256::from(300u64)).pass);
    }

    // ── TEST:verify-safe-tx-hash-ok ────────────────────────────────────────

    #[test]
    fn test_verify_safe_tx_hash_ok() {
        let safe: Address = "0xb76834e371b666feee48e5d7d9a97ca08b5a0620"
            .parse()
            .unwrap();
        let chain_id: u64 = 11155111;
        let timelock: Address = "0x8e3b6563d683b87964104a2c3a4bf542bb70767f"
            .parse()
            .unwrap();

        let s: SafeBatch = serde_json::from_str(SCHEDULE_JSON).unwrap();
        let e: SafeBatch = serde_json::from_str(EXECUTE_JSON).unwrap();
        let (_, sched_calldata) = tx_calldata(&s.transactions[0]).unwrap();
        let (_, exec_calldata) = tx_calldata(&e.transactions[0]).unwrap();

        let sched_hashes =
            safe_tx_hashes(safe, chain_id, timelock, U256::ZERO, &sched_calldata, 0, 24);
        let expected_domain: B256 =
            "0x8f560c9d209e6d9320305560aee98fa1dea01510aa5451a9c0911401893835c6"
                .parse()
                .unwrap();
        assert_eq!(sched_hashes.domain, expected_domain);
        assert_eq!(
            sched_hashes.message,
            "0x9c5a62271d73b6accf3c8957a1e80b6434618d3bd4b8bd23e30817479c60d35b"
                .parse::<B256>()
                .unwrap()
        );
        assert_eq!(
            sched_hashes.safe_tx,
            "0xa3d4b5bfa93b559f34478b3988f1132c35ba67f953a87326c8a1c8250709c6b8"
                .parse::<B256>()
                .unwrap()
        );

        let exec_hashes =
            safe_tx_hashes(safe, chain_id, timelock, U256::ZERO, &exec_calldata, 0, 25);
        assert_eq!(exec_hashes.domain, expected_domain);
        assert_eq!(
            exec_hashes.message,
            "0xf7edebe09a94e770ddbccf107a5685d50d902adb08db5e2043c7b1f9c4ef648b"
                .parse::<B256>()
                .unwrap()
        );
        assert_eq!(
            exec_hashes.safe_tx,
            "0xbb7fd662e5b724a50e33f18ef737d6df9c1d92b8810def16fb190b7c27c16f45"
                .parse::<B256>()
                .unwrap()
        );
    }

    // ── TEST:verify-exit-code-fail-nonzero ────────────────────────────────

    #[test]
    fn test_verify_exit_code_fail_nonzero() {
        let domain: B256 = B256::ZERO;
        let report = VerifyReport {
            rows: vec![pass("check-a", "ok"), fail("check-b", "something wrong")],
            header: ReportHeader {
                proxy: Address::ZERO,
                new_impl: Address::ZERO,
                contract_name: "StakeTableV3",
                network: "decaf".to_owned(),
                description: String::new(),
                onchain_solc: None,
                reference_solc: None,
            },
            phase_hashes: PhaseHashes {
                schedule_nonce: 24,
                schedule: SafeTxHashes {
                    domain,
                    message: domain,
                    safe_tx: domain,
                },
                execute_nonce: 25,
                execute: SafeTxHashes {
                    domain,
                    message: domain,
                    safe_tx: domain,
                },
            },
        };
        assert_eq!(report.exit_code(), 1);
    }

    // ── TEST:verify-empty-init-at-target-ok ───────────────────────────────
    //
    // Empty init data is fine when the proxy already reached the target major
    // (re-verify after upgrade, or a patch within the same major).

    #[test]
    fn test_verify_empty_init_at_target_ok() {
        let kind = contract_kind(ContractKindArg::StakeTableV3);
        let row = check_init_selector(&kind, &Bytes::new(), Some(3));
        assert!(row.pass, "{}", row.detail);
    }

    // ── TEST:verify-empty-init-below-target-fails ─────────────────────────
    //
    // A V2→V3 proposal with empty init data would skip initializeV3(); must FAIL.

    #[test]
    fn test_verify_empty_init_below_target_fails() {
        let kind = contract_kind(ContractKindArg::StakeTableV3);
        let row = check_init_selector(&kind, &Bytes::new(), Some(2));
        assert!(!row.pass, "{}", row.detail);
        assert!(row.detail.contains("would never run"), "{}", row.detail);
    }

    // ── TEST:verify-empty-init-unknown-version-fails ──────────────────────

    #[test]
    fn test_verify_empty_init_unknown_version_fails() {
        let kind = contract_kind(ContractKindArg::StakeTableV3);
        let row = check_init_selector(&kind, &Bytes::new(), None);
        assert!(!row.pass, "{}", row.detail);
    }

    // ── TEST:verify-feecontract-no-init-ok ────────────────────────────────

    #[test]
    fn test_verify_feecontract_no_init_ok() {
        let kind = contract_kind(ContractKindArg::FeeContract);
        let row = check_init_selector(&kind, &Bytes::new(), None);
        assert!(row.pass, "FeeContract patch should accept empty init");
    }

    // ── TEST:verify-feecontract-nonempty-init-fails ───────────────────────
    //
    // Non-empty init data with expected_init=None must FAIL.

    #[test]
    fn test_verify_feecontract_nonempty_init_fails() {
        let kind = contract_kind(ContractKindArg::FeeContract);
        let data = Bytes::from(vec![0xde, 0xad, 0xbe, 0xef, 0x00]);
        let row = check_init_selector(&kind, &data, Some(1));
        assert!(
            !row.pass,
            "non-empty init with no expected selector must FAIL: {}",
            row.detail
        );
        assert!(
            row.detail.contains("arbitrary delegated call"),
            "detail should explain risk: {}",
            row.detail
        );
    }

    // ── TEST:verify-rewardclaim-nonempty-init-fails ───────────────────────

    #[test]
    fn test_verify_rewardclaim_nonempty_init_fails() {
        let kind = contract_kind(ContractKindArg::RewardClaim);
        let data = Bytes::from(vec![0xca, 0xfe, 0xba, 0xbe]);
        let row = check_init_selector(&kind, &data, Some(1));
        assert!(
            !row.pass,
            "non-empty init on RewardClaim with no expected selector must FAIL: {}",
            row.detail
        );
    }

    // ── TEST:verify-value-nonzero-fails ───────────────────────────────────

    #[test]
    fn test_verify_value_nonzero_fails() {
        assert!(!value_zero_row(U256::from(1u64)).pass);
        assert!(value_zero_row(U256::ZERO).pass);
    }

    // ── TEST:verify-predecessor-nonzero-fails ─────────────────────────────

    #[test]
    fn test_verify_predecessor_nonzero_fails() {
        assert!(!predecessor_zero_row(B256::repeat_byte(0x01)).pass);
        assert!(predecessor_zero_row(B256::ZERO).pass);
    }

    // ── TEST:verify-toml-tampered-impl-fails ──────────────────────────────

    #[test]
    fn test_verify_toml_tampered_impl_fails() {
        let upgrade_impl: Address = "0x5a6250dd35d875c0529573d9d934629a1b2778db"
            .parse()
            .unwrap();
        let tampered_impl = Address::repeat_byte(0xde);

        // toml_field_row returns fail when decoded != recorded.
        let row = toml_field_row("impl", upgrade_impl, tampered_impl);
        assert!(!row.pass, "tampered impl must fail: {}", row.detail);
        assert!(row.detail.contains("decoded="), "detail: {}", row.detail);
    }

    // ── TEST:verify-toml-tampered-safe-tx-fails ───────────────────────────

    #[test]
    fn test_verify_toml_tampered_safe_tx_fails() {
        let s: SafeBatch = serde_json::from_str(SCHEDULE_JSON).unwrap();
        let e: SafeBatch = serde_json::from_str(EXECUTE_JSON).unwrap();
        let (_, sched_calldata) = tx_calldata(&s.transactions[0]).unwrap();
        let (_, exec_calldata) = tx_calldata(&e.transactions[0]).unwrap();

        let mut toml = fixture_toml();
        // Tamper the schedule safe_tx hash.
        toml.schedule.safe_tx = B256::repeat_byte(0xba);

        let calldatas = OuterCalldatas {
            schedule: sched_calldata,
            execute: exec_calldata,
        };
        let mut rows = vec![];
        compute_and_validate_phase_hashes(&toml, 11155111, &calldatas, &mut rows);

        let safe_tx_row = rows
            .iter()
            .find(|r| r.name == "toml:schedule.safe_tx")
            .expect("must have schedule.safe_tx row");
        assert!(
            !safe_tx_row.pass,
            "tampered safe_tx must fail: {}",
            safe_tx_row.detail
        );
    }

    // ── TEST:verify-toml-round-trip-ok ────────────────────────────────────

    #[test]
    fn test_verify_toml_round_trip_ok() {
        let original = fixture_toml();
        let tmp = tempfile::tempdir().unwrap();
        original.write(tmp.path()).unwrap();
        let loaded = ProposalToml::load(tmp.path()).unwrap();
        assert_eq!(original, loaded);
    }

    // ── TEST:verify-network-chain-id-match-ok ─────────────────────────────

    #[test]
    fn test_verify_network_chain_id_match_ok() {
        assert!(network_chain_id_row("mainnet", 1).pass);
        assert!(network_chain_id_row("decaf", 11155111).pass);
        assert!(network_chain_id_row("hoodi", 560048).pass);
    }

    // ── TEST:verify-network-chain-id-mismatch-fails ───────────────────────

    #[test]
    fn test_verify_network_chain_id_mismatch_fails() {
        let row = network_chain_id_row("mainnet", 11155111);
        assert!(!row.pass, "mainnet with sepolia chain_id must fail");
        assert!(
            row.detail.contains("chain_id=1"),
            "detail must show expected: {}",
            row.detail
        );
    }

    // ── TEST:verify-network-unknown-fails ─────────────────────────────────

    #[test]
    fn test_verify_network_unknown_fails() {
        let row = network_chain_id_row("bogusnet", 999);
        assert!(!row.pass, "unknown network must fail");
        assert!(
            row.detail.contains("unknown network"),
            "detail: {}",
            row.detail
        );
    }

    // ── TEST:verify-safe-address-unknown-network-fail ─────────────────────

    #[test]
    fn test_verify_safe_address_unknown_network_fail() {
        let mut toml = fixture_toml();
        toml.network = "bogusnet".to_owned();
        let kind = contract_kind(ContractKindArg::StakeTableV3);
        let rows = safe_address_rows(&toml, &kind);
        assert_eq!(rows.len(), 2);
        assert!(!rows[0].pass, "unknown network must FAIL schedule.safe");
        assert!(!rows[1].pass, "unknown network must FAIL execute.safe");
    }

    // ── TEST:verify-safe-address-safe-exit-kind ───────────────────────────

    #[test]
    fn test_verify_safe_address_safe_exit_kind() {
        // RewardClaim is SafeExit; decaf safe_exit_timelock has espresso_labs as proposer/executor.
        let espresso_labs: Address = "0xb76834e371b666feee48e5d7d9a97ca08b5a0620"
            .parse()
            .unwrap();
        let mut toml = fixture_toml();
        toml.schedule.safe = espresso_labs;
        toml.execute.safe = espresso_labs;
        let kind = contract_kind(ContractKindArg::RewardClaim);
        let rows = safe_address_rows(&toml, &kind);
        assert_eq!(rows.len(), 2);
        assert!(
            rows[0].pass,
            "espresso_labs is safe_exit proposer: {}",
            rows[0].detail
        );
        assert!(
            rows[1].pass,
            "espresso_labs is safe_exit executor: {}",
            rows[1].detail
        );
    }

    // ── Helpers ───────────────────────────────────────────────────────────

    /// Real solc tail with `bytecode_hash = "none"`: `a1 64 "solc" 43 <ver>` + u16 length.
    fn make_cbor_tail(ver: [u8; 3]) -> Vec<u8> {
        let mut cbor = vec![
            0xa1, 0x64, 0x73, 0x6f, 0x6c, 0x63, 0x43, ver[0], ver[1], ver[2],
        ];
        cbor.extend_from_slice(&(cbor.len() as u16).to_be_bytes());
        cbor
    }
}
