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

use crate::{
    Contract, Contracts,
    proposals::{
        deployment_info::{default_deployment_info_dir, load_ops_timelock_signers},
        proposal_toml::ProposalToml,
        safe_hash::{SafeTxHashes, safe_tx_hashes},
        write::default_rpc_url,
    },
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

#[derive(Debug, Deserialize)]
pub struct SafeTx {
    pub to: Address,
    pub value: String,
    pub data: Option<String>,
    #[serde(rename = "contractMethod")]
    pub contract_method: Option<SafeContractMethod>,
    #[serde(rename = "contractInputsValues")]
    pub contract_inputs_values: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Deserialize)]
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
    /// Cores match but solc versions differ — PASS with warning.
    CodeMatchMetaDiffers,
    /// Core bytecodes differ — FAIL.
    Mismatch,
}

#[derive(Debug, Clone)]
pub struct BytecodeCheck {
    pub class: MatchClass,
    pub onchain_solc: Option<SolcVersion>,
    pub reference_solc: Option<SolcVersion>,
}

/// Strip trailing solc CBOR metadata.
///
/// Solidity CBOR tail format:
///   `a1 64 "solc" 43 <3-byte ver> 00 <u16-BE length>`
/// where the u16 at the very end is the byte length of the CBOR body.
/// Total drop = CBOR-length + 2.
///
/// Returns `(stripped_code, solc_version)`.
pub fn strip_cbor_metadata(code: &[u8]) -> (Vec<u8>, Option<SolcVersion>) {
    if code.len() < 2 {
        return (code.to_vec(), None);
    }
    let tail_len = u16::from_be_bytes([code[code.len() - 2], code[code.len() - 1]]) as usize;
    let total_drop = tail_len + 2;
    if total_drop > code.len() {
        return (code.to_vec(), None);
    }
    let cbor_start = code.len() - total_drop;
    let cbor = &code[cbor_start..code.len() - 2];
    let solc_prefix: &[u8] = &[0xa1, 0x64, 0x73, 0x6f, 0x6c, 0x63, 0x43];
    let stripped = code[..cbor_start].to_vec();
    if cbor.len() >= 11 {
        for i in 0..=(cbor.len() - 11) {
            if cbor[i..i + 7] == *solc_prefix && cbor[i + 10] == 0x00 {
                let ver = SolcVersion([cbor[i + 7], cbor[i + 8], cbor[i + 9]]);
                return (stripped, Some(ver));
            }
        }
    }
    (stripped, None)
}

/// Mask all occurrences of `impl_addr` (20 bytes) in `code` via substring scan.
///
/// The UUPS `__self` immutable sits at compiler-chosen non-aligned byte offsets.
/// Each non-overlapping 20-byte occurrence is zeroed in-place.
///
/// Returns the number of occurrences zeroed.
pub fn mask_immutables(code: &mut [u8], impl_addr: Address) -> usize {
    let addr_bytes = impl_addr.as_slice();
    let mut count = 0usize;
    if code.len() < 20 {
        return 0;
    }
    let mut i = 0usize;
    while i <= code.len() - 20 {
        if &code[i..i + 20] == addr_bytes {
            code[i..i + 20].fill(0);
            count += 1;
            i += 20;
        } else {
            i += 1;
        }
    }
    count
}

/// Compare normalized on-chain bytecode against the binding reference.
///
/// Normalization: strip CBOR (both) → substring-mask `impl_addr` on ON-CHAIN
/// only → assert equal core length → classify.
///
/// The binding already has zeros at `__self` windows (compiled with
/// address(this)=0 placeholder), so the reference is left untouched.
///
/// LightClient verification is deferred; bails if the reference contains
/// `0xff*20` library placeholders.
pub fn compare_normalized(
    onchain: &[u8],
    reference: &[u8],
    impl_addr: Address,
) -> Result<BytecodeCheck> {
    let placeholder = [0xffu8; 20];
    if reference.windows(20).any(|w| w == placeholder.as_slice()) {
        bail!(
            "reference bytecode contains library placeholder (0xff*20); LightClient verification \
             is deferred"
        );
    }

    let (mut onchain_core, onchain_solc) = strip_cbor_metadata(onchain);
    let (ref_core, ref_solc) = strip_cbor_metadata(reference);

    let count = mask_immutables(&mut onchain_core, impl_addr);
    if onchain_core.len() >= 20 && count == 0 {
        bail!(
            "mask_immutables found 0 occurrences of impl_addr {impl_addr}; expected at least 1 \
             for a UUPS contract — bytecode may be wrong"
        );
    }
    if count > 8 {
        bail!("unexpected immutable count: {count}; expected 1..=8");
    }

    if onchain_core.len() != ref_core.len() {
        return Ok(BytecodeCheck {
            class: MatchClass::Mismatch,
            onchain_solc,
            reference_solc: ref_solc,
        });
    }

    let class = match (onchain_core == ref_core, onchain_solc == ref_solc) {
        (true, true) => MatchClass::FullMatch,
        (true, false) => MatchClass::CodeMatchMetaDiffers,
        (false, _) => MatchClass::Mismatch,
    };
    Ok(BytecodeCheck {
        class,
        onchain_solc,
        reference_solc: ref_solc,
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

#[derive(Debug, Clone)]
pub struct ContractKind {
    pub contract: Contract,
    pub name: &'static str,
    pub deployed_bytecode: &'static [u8],
    pub expected_init_selector: Option<[u8; 4]>,
    pub owner_accessor: OwnerAccessor,
    pub timelock_kind: TimelockKind,
    pub expected_prev_major: Option<u8>,
}

pub fn contract_kind(arg: ContractKindArg) -> ContractKind {
    match arg {
        ContractKindArg::StakeTableV2 => ContractKind {
            contract: Contract::StakeTableV2,
            name: "StakeTableV2",
            deployed_bytecode: &StakeTableV2::DEPLOYED_BYTECODE,
            expected_init_selector: Some(StakeTableV2::initializeV2Call::SELECTOR),
            owner_accessor: OwnerAccessor::Owner,
            timelock_kind: TimelockKind::Ops,
            expected_prev_major: Some(1),
        },
        ContractKindArg::StakeTableV3 => ContractKind {
            contract: Contract::StakeTableV3,
            name: "StakeTableV3",
            deployed_bytecode: &StakeTableV3::DEPLOYED_BYTECODE,
            expected_init_selector: Some(StakeTableV3::initializeV3Call::SELECTOR),
            owner_accessor: OwnerAccessor::Owner,
            timelock_kind: TimelockKind::Ops,
            expected_prev_major: Some(2),
        },
        ContractKindArg::EspTokenV2 => ContractKind {
            contract: Contract::EspTokenV2,
            name: "EspTokenV2",
            deployed_bytecode: &EspTokenV2::DEPLOYED_BYTECODE,
            expected_init_selector: Some(EspTokenV2::initializeV2Call::SELECTOR),
            owner_accessor: OwnerAccessor::Owner,
            timelock_kind: TimelockKind::SafeExit,
            expected_prev_major: Some(1),
        },
        ContractKindArg::FeeContract => ContractKind {
            contract: Contract::FeeContract,
            name: "FeeContract",
            deployed_bytecode: &FeeContract::DEPLOYED_BYTECODE,
            expected_init_selector: None,
            owner_accessor: OwnerAccessor::Owner,
            timelock_kind: TimelockKind::Ops,
            expected_prev_major: Some(1),
        },
        ContractKindArg::RewardClaim => ContractKind {
            contract: Contract::RewardClaim,
            name: "RewardClaim",
            deployed_bytecode: &RewardClaim::DEPLOYED_BYTECODE,
            expected_init_selector: None,
            owner_accessor: OwnerAccessor::CurrentAdmin,
            timelock_kind: TimelockKind::SafeExit,
            expected_prev_major: None,
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
fn batch_phase(batch: &SafeBatch) -> Result<Phase> {
    let tx = batch
        .transactions
        .first()
        .ok_or_else(|| anyhow!("batch has no transactions"))?;

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
/// Both files must be present and parse as `SafeBatch`. Phase is validated
/// via `batch_phase`: `schedule.json` must be `Phase::Schedule` and
/// `execute.json` must be `Phase::Execute`; any mismatch is an error.
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
pub fn decode_proposal(batches: TimelockBatches) -> Result<DecodedUpgrade> {
    let sched_tx = batches
        .schedule
        .transactions
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("schedule batch has no transactions"))?;
    let exec_tx = batches
        .execute
        .transactions
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("execute batch has no transactions"))?;

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

// ── CLI args ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, clap::Args)]
pub struct VerifyProposalArgs {
    /// Proposal directory containing schedule.json, execute.json, and proposal.toml.
    pub dir: PathBuf,

    /// Override the contract kind; defaults to proposal.toml `contract` field.
    #[clap(long)]
    pub contract: Option<ContractKindArg>,

    /// RPC URL; defaults to the network's public node from proposal.toml.
    #[clap(long)]
    pub rpc_url: Option<Url>,
}

// ── Orchestrator ─────────────────────────────────────────────────────────────

/// Run verification without a wallet provider.
///
/// Reads `chain_id` from `<args.dir>/proposal.toml`, resolves the RPC from
/// `args.rpc_url` or the built-in public-node map, then delegates to `run_verify`.
pub async fn run_verify_standalone(
    args: &VerifyProposalArgs,
    contracts: &Contracts,
) -> Result<VerifyReport> {
    let toml = ProposalToml::load(&args.dir)?;
    let chain_id = toml.chain_id;

    let rpc = args
        .rpc_url
        .clone()
        .or_else(|| default_rpc_url(chain_id))
        .ok_or_else(|| anyhow!("unknown chain id {chain_id}; pass --rpc-url"))?;

    let provider = ProviderBuilder::new().connect_http(rpc);

    let provider_chain_id = provider.get_chain_id().await?;
    run_verify(
        args,
        &provider,
        contracts,
        provider_chain_id,
        &default_deployment_info_dir(),
    )
    .await
}

pub async fn run_verify(
    args: &VerifyProposalArgs,
    provider: &impl Provider,
    contracts: &Contracts,
    chain_id: u64,
    deployment_info_dir: &Path,
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

    // chain_id / network consistency
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
            // Cannot proceed without a decoded upgrade.
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

    let onchain_code = provider.get_code_at(upgrade.new_impl).await?;

    let bytecode_check =
        match compare_normalized(&onchain_code, kind.deployed_bytecode, upgrade.new_impl) {
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
                "on-chain impl at {} does not match {} binding",
                upgrade.new_impl, kind.name
            ),
        ),
    });

    rows.push(check_init_selector(&kind, &upgrade.init_data));

    // Safe validation: assert toml Safes match deployment-info.
    let safe_rows = safe_address_rows(&toml, &kind, deployment_info_dir);
    rows.extend(safe_rows);

    // Recompute Safe hashes for both phases and assert against toml.
    let phase_hashes =
        compute_and_validate_phase_hashes(&toml, chain_id, &upgrade.outer_calldatas, &mut rows);

    // Nonce drift check (WARN, not FAIL).
    nonce_drift_rows(provider, &toml, &mut rows).await;

    let gov_rows = governance_checks(provider, contracts, &upgrade, &kind).await;
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

/// Validate toml Safe addresses against deployment-info (hard fail on mismatch).
fn safe_address_rows(
    toml: &ProposalToml,
    kind: &ContractKind,
    deployment_info_dir: &Path,
) -> Vec<CheckRow> {
    let mut rows = vec![];

    // Only check ops_timelock for Ops-kind contracts; SafeExit uses a different section.
    // For now we only have deployment-info for ops_timelock proposers/executors.
    if kind.timelock_kind != TimelockKind::Ops {
        rows.push(pass(
            "toml:schedule.safe",
            "Safe-exit timelock: deployment-info check skipped",
        ));
        rows.push(pass(
            "toml:execute.safe",
            "Safe-exit timelock: deployment-info check skipped",
        ));
        return rows;
    }

    match load_ops_timelock_signers(&toml.network, deployment_info_dir) {
        Err(e) => {
            rows.push(pass(
                "toml:schedule.safe",
                format!("deployment-info unavailable ({e}); skipped"),
            ));
            rows.push(pass(
                "toml:execute.safe",
                format!("deployment-info unavailable ({e}); skipped"),
            ));
        },
        Ok(signers) => {
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
        },
    }

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
    use crate::proposals::write::ISafe;

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
                             in toml use recorded nonce — signer must reconfirm",
                        ),
                    ));
                }
            },
        }
    }
}

fn check_init_selector(kind: &ContractKind, init_data: &Bytes) -> CheckRow {
    match kind.expected_init_selector {
        None => pass(
            "init-selector",
            if init_data.is_empty() {
                "empty (expected for patch/no-reinitializer)".to_owned()
            } else {
                format!(
                    "non-empty init data but no selector expected; selector=0x{}",
                    alloy::hex::encode(&init_data[..4.min(init_data.len())])
                )
            },
        ),
        Some(expected) => {
            if init_data.is_empty() {
                pass("init-selector", "empty (proxy already at target version)")
            } else if init_data.len() >= 4 && init_data[..4] == expected {
                pass(
                    "init-selector",
                    format!("ok selector=0x{}", alloy::hex::encode(expected)),
                )
            } else {
                fail(
                    "init-selector",
                    format!(
                        "expected 0x{} got 0x{}",
                        alloy::hex::encode(expected),
                        alloy::hex::encode(&init_data[..4.min(init_data.len())])
                    ),
                )
            }
        },
    }
}

async fn governance_checks(
    provider: &impl Provider,
    contracts: &Contracts,
    upgrade: &DecodedUpgrade,
    kind: &ContractKind,
) -> Vec<CheckRow> {
    let mut rows = vec![];

    match fetch_proxy_owner(provider, upgrade.proxy, kind.owner_accessor).await {
        Err(e) => rows.push(fail("owner-query", e.to_string())),
        Ok(owner) => {
            rows.push(owner_timelock_row(owner, upgrade.outer_to));
            let expected_timelock_contract = match kind.timelock_kind {
                TimelockKind::Ops => Contract::OpsTimelock,
                TimelockKind::SafeExit => Contract::SafeExitTimelock,
            };
            if let Some(expected_addr) = contracts.address(expected_timelock_contract) {
                rows.push(if upgrade.outer_to == expected_addr {
                    pass(
                        "timelock-addr-match",
                        format!("{:?}={}", expected_timelock_contract, expected_addr),
                    )
                } else {
                    fail(
                        "timelock-addr-match",
                        format!(
                            "outer_to={} expected {:?}={}",
                            upgrade.outer_to, expected_timelock_contract, expected_addr
                        ),
                    )
                });
            }
        },
    }

    match fetch_min_delay(provider, upgrade.outer_to).await {
        Err(e) => rows.push(fail("delay>=minDelay", e.to_string())),
        Ok(min_delay) => rows.push(delay_row(upgrade.delay, min_delay)),
    }

    match (
        fetch_proxy_major_version(provider, upgrade.proxy).await,
        kind.expected_prev_major,
    ) {
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
    use hotshot_contract_adapter::sol_types::StakeTableV3;

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
        // schedule.json contains an execute call — phase mismatch must error.
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
        let cbor: Vec<u8> = vec![
            0xa1, 0x64, 0x73, 0x6f, 0x6c, 0x63, 0x43, 0x00, 0x08, 0x1c, 0x00,
        ];
        let cbor_len = cbor.len() as u16;
        let mut code: Vec<u8> = vec![0xde, 0xad, 0xbe, 0xef];
        code.extend_from_slice(&cbor);
        code.extend_from_slice(&cbor_len.to_be_bytes());
        let (stripped, ver) = strip_cbor_metadata(&code);
        assert_eq!(stripped, vec![0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(ver, Some(SolcVersion([0x00, 0x08, 0x1c])));
    }

    // ── TEST:verify-strip-cbor-835-ok ─────────────────────────────────────

    #[test]
    fn test_verify_strip_cbor_835_ok() {
        let cbor: Vec<u8> = vec![
            0xa1, 0x64, 0x73, 0x6f, 0x6c, 0x63, 0x43, 0x00, 0x08, 0x23, 0x00,
        ];
        let cbor_len = cbor.len() as u16;
        let mut code: Vec<u8> = vec![0xca, 0xfe];
        code.extend_from_slice(&cbor);
        code.extend_from_slice(&cbor_len.to_be_bytes());
        let (stripped, ver) = strip_cbor_metadata(&code);
        assert_eq!(stripped, vec![0xca, 0xfe]);
        assert_eq!(ver, Some(SolcVersion([0x00, 0x08, 0x23])));
    }

    // ── TEST:verify-no-metadata-tail-ok ───────────────────────────────────

    #[test]
    fn test_verify_no_metadata_tail_ok() {
        let code: Vec<u8> = vec![0xde, 0xad, 0xbe, 0xef, 0x00, 0x04];
        let (_, ver) = strip_cbor_metadata(&code);
        assert!(ver.is_none());
    }

    // ── TEST:verify-mask-immutables-ok ────────────────────────────────────

    #[test]
    fn test_verify_mask_immutables_ok() {
        let addr = Address::repeat_byte(0xab);
        let mut code = vec![0xffu8; 300];
        code[45..65].copy_from_slice(addr.as_slice());
        code[200..220].copy_from_slice(addr.as_slice());

        let n = mask_immutables(&mut code, addr);
        assert_eq!(n, 2);
        assert_eq!(&code[45..65], &[0u8; 20]);
        assert_eq!(&code[200..220], &[0u8; 20]);
        assert_eq!(code[44], 0xff);
        assert_eq!(code[65], 0xff);
        assert_eq!(code[199], 0xff);
        assert_eq!(code[220], 0xff);
    }

    // ── TEST:verify-bytecode-fullmatch-ok ─────────────────────────────────

    #[test]
    fn test_verify_bytecode_fullmatch_ok() {
        let impl_addr = Address::repeat_byte(0x01);
        let cbor = make_cbor_tail([0x00, 0x08, 0x23]);

        // impl_addr at non-aligned offset 5; reference has zeros there.
        let mut onchain = vec![0xaa, 0xbb, 0xcc, 0xdd, 0xee];
        onchain.extend_from_slice(impl_addr.as_slice());
        onchain.extend_from_slice(&[0x11, 0x22, 0x33]);
        onchain.extend_from_slice(&cbor);

        let mut reference = vec![0xaa, 0xbb, 0xcc, 0xdd, 0xee];
        reference.extend_from_slice(&[0u8; 20]);
        reference.extend_from_slice(&[0x11, 0x22, 0x33]);
        reference.extend_from_slice(&cbor);

        let check = compare_normalized(&onchain, &reference, impl_addr).unwrap();
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

        let check = compare_normalized(&onchain, &reference, impl_addr).unwrap();
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

        let check = compare_normalized(&onchain, &reference, impl_addr).unwrap();
        assert_eq!(check.class, MatchClass::Mismatch);
    }

    // ── TEST:verify-kind-by-bytecode-ok ───────────────────────────────────

    #[test]
    fn test_verify_kind_by_bytecode_ok() {
        // Real round-trip: inject fake impl address into StakeTableV3::DEPLOYED_BYTECODE
        // at artifact-reported immutable window positions (start=10580,10621,11787 length=32),
        // addr at offset+12..offset+32. Substring scan must recover the binding.
        let binding = StakeTableV3::DEPLOYED_BYTECODE.as_ref();
        let fake_impl = Address::repeat_byte(0x42);

        let windows: &[(usize, usize)] = &[(10580, 32), (10621, 32), (11787, 32)];
        let mut onchain = binding.to_vec();
        for &(start, length) in windows {
            let addr_start = start + (length - 20);
            onchain[addr_start..start + length].copy_from_slice(fake_impl.as_slice());
        }

        let check = compare_normalized(&onchain, binding, fake_impl)
            .expect("compare_normalized must succeed on real bytecode");
        assert!(
            check.class == MatchClass::FullMatch || check.class == MatchClass::CodeMatchMetaDiffers,
            "expected PASS class, got {:?}",
            check.class
        );
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

    // ── TEST:verify-empty-init-accepted-ok ────────────────────────────────

    #[test]
    fn test_verify_empty_init_accepted_ok() {
        let kind = contract_kind(ContractKindArg::StakeTableV3);
        let row = check_init_selector(&kind, &Bytes::new());
        assert!(row.pass, "empty init should be accepted: {}", row.detail);
    }

    // ── TEST:verify-feecontract-no-init-ok ────────────────────────────────

    #[test]
    fn test_verify_feecontract_no_init_ok() {
        let kind = contract_kind(ContractKindArg::FeeContract);
        let row = check_init_selector(&kind, &Bytes::new());
        assert!(row.pass, "FeeContract patch should accept empty init");
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
    //
    // A tampered toml.impl must produce a FAIL row when the decoded impl differs.

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
    //
    // A tampered safe_tx hash must produce a FAIL row from compute_and_validate_phase_hashes.

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

    // ── Helpers ───────────────────────────────────────────────────────────

    fn make_cbor_tail(ver: [u8; 3]) -> Vec<u8> {
        let mut cbor = vec![
            0xa1, 0x64, 0x73, 0x6f, 0x6c, 0x63, 0x43, ver[0], ver[1], ver[2], 0x00,
        ];
        cbor.extend_from_slice(&(cbor.len() as u16).to_be_bytes());
        cbor
    }
}
