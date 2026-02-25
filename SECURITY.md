# Security Policy

## Reporting a Vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

To report a vulnerability, please use one of the following channels:

- **Email**: security@espressosys.com
- **GitHub Security Advisories**:
  [https://github.com/EspressoSystems/espresso-network/security/advisories/new](https://github.com/EspressoSystems/espresso-network/security/advisories/new)

We will acknowledge your report within **2 business days** and aim to provide an initial assessment within **5 business
days**.

---

## Safe Harbor - Researcher Authorization

Espresso Systems authorizes good-faith security research on the assets listed in the **Scope** section below.

If you act in good faith and comply with this policy, we commit to:

- Not pursuing or supporting any legal action against you related to your research
- Not referring or reporting you to law enforcement for your research activities
- Working with you to understand and resolve the issue quickly

This authorization does not extend to: denial of service attacks, social engineering, phishing, or testing against
systems not listed in scope. Testing on mainnet with real user funds is **strictly prohibited**.

---

## SEAL Whitehat Safe Harbor Agreement (Active Exploits)

Espresso Systems has adopted the **Security Alliance (SEAL) Whitehat Safe Harbor Agreement v3.0.0** for active blackhat
exploit scenarios.

**This agreement covers only active, in-progress exploits against in-scope contracts.** It does not cover general
security research, which is covered by the researcher authorization section above.

Under the Safe Harbor Agreement, if you discover an active blackhat exploit in progress, you may be authorized to
intervene to rescue protocol funds and return them to the Asset Recovery Address (ARA) in exchange for a reward.

**Before acting under the Safe Harbor Agreement, you MUST verify:**

1. There is an active, urgent exploit already in progress (not hypothetical)
2. Normal bug bounty disclosure is not practical due to time constraints
3. You are not the party who initiated the exploit
4. Your intervention is net-beneficial - it reduces total losses
5. You have the technical expertise to act without causing further harm
6. You and all funds/addresses used are free from OFAC sanctions
7. You have read and understood the [full SEAL agreement](https://github.com/security-alliance/safe-harbor)
8. The adoption has been confirmed via the SEAL SafeHarborRegistry on-chain

**Asset Recovery Address (ARA) (Ethereum Mainnet) :** `0x5e37B8038615EF3D75cf28b5982C4CBF065401fB`

**Asset Recovery Address (ARA) (Arbitrum One) :** `0x5e37B8038615EF3D75cf28b5982C4CBF065401fB`

**Reward**: Up to 10% of rescued assets, capped at a maximum of $1,000,000. All funds must be returned to the ARA within
**6 hours** of rescue. If you cannot meet this deadline, notify us immediately at security@espressosys.com.

The full agreement text is maintained by SEAL at:
https://github.com/security-alliance/safe-harbor/blob/main/documents/agreement.md

---

## Scope

### In Scope - Smart Contracts

The following deployed contracts on **Ethereum Mainnet** and **Arbitrum** are in scope:

| Contract                                  | Description                                               |
| ----------------------------------------- | --------------------------------------------------------- |
| `StakeTableV2`                            | Validator registration, staking, delegations, withdrawals |
| `LightClientV3` / `LightClientArbitrumV3` | HotShot state proof verification, block commitments       |
| `EspTokenV2`                              | ESP ERC-20 token                                          |
| `RewardClaim`                             | Validator reward distribution via merkle proof            |
| `FeeContract`                             | Builder fee deposits                                      |
| `OpsTimelock` / `SafeExitTimelock`        | Timelock governance contracts                             |

> Replace with actual deployed addresses once confirmed on mainnet.

### In Scope - Protocol & Infrastructure

- Sequencer node software ([`sequencer/`](./sequencer/))
- HotShot consensus library ([`crates/hotshot/`](./crates/hotshot/))
- Public-facing sequencer APIs

### Out of Scope

- Testnet deployments (please still report serious findings)
- Third-party infrastructure (RPC providers, bridges not operated by Espresso)
- Frontend/UI issues without demonstrable security impact
- Theoretical attacks without a working proof of concept
- Known issues listed in previous audit reports (see [`audits/`](./audits/))
- Attacks that require physical access to infrastructure or social engineering of team members

---

## Severity Classification

| Severity     | Examples                                                                                                                                                    |
| ------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Critical** | Unauthorized token minting, complete drain of staked funds, forged ZK proof accepted by LightClient, validator set manipulation, consensus safety violation |
| **High**     | Privilege escalation, logic bugs causing significant fund loss, upgrade storage corruption                                                                  |
| **Medium**   | Denial of service, griefing attacks with meaningful economic impact                                                                                         |
| **Low**      | Minor logic errors, informational mismatches without financial impact                                                                                       |

---

## Disclosure Policy

This program follows **coordinated disclosure**:

- Please allow us **90 days** to investigate and patch before public disclosure
- We will coordinate disclosure timing with you and credit you for the discovery
- If a fix requires longer than 90 days, we will communicate this to you with an updated timeline
- We will not publicly disclose details of your report without your permission

---

## Audit History

Previous security audits are available in the [`audits/`](./audits/) directory and at
[https://github.com/EspressoSystems/Espresso-audits](https://github.com/EspressoSystems/Espresso-audits).

---

## Rewards

Rewards are paid at our discretion based on severity, impact, and quality of the report. Reward amounts will be
communicated directly during the disclosure process.

We do not reward:

- Reports for issues already known or previously disclosed
- Reports lacking a proof of concept for critical/high severity issues
- Duplicate reports (first report wins)
- Issues with no realistic security impact

---

## Contact

- **Security email**: security@espressosys.com
- **GitHub Security Advisories**:
  [Report privately via GitHub](https://github.com/EspressoSystems/espresso-network/security/advisories/new)
- **SEAL Safe Harbor questions**: safeharbor@securityalliance.org
