# Stake Re-delegation

## 1. Overview & Motivation

Currently, moving stake between validators requires three transactions and a waiting period:

1. `undelegate(fromValidator, amount)` -- begins escrow
2. Wait for escrow period to expire
3. `claimWithdrawal()` -- tokens returned to delegator
4. `delegate(toValidator, amount)` -- stake with new validator

This locks capital during the escrow period, penalizing delegators who want to rebalance.

**Redelegation** replaces this with a single `redelegate(fromValidator, toValidator, amount)` transaction. Tokens move
atomically between validators with no escrow, no capital lockup, and no change to `activeStake`.

### Why no escrow?

Escrow exists to prevent delegators from escaping slashing. Without slashing (current state), escrow on redelegation
serves no purpose. When slashing lands in V4, escrow is added to redelegation at that point.

## 2. Contract Changes (StakeTableV3)

### Versioning

- `StakeTableV3 is StakeTableV2`
- `reinitializer(3)`
- `getVersion()` returns `(3, 0, 0)`
- Clean boundary: V2 = current, V3 = redelegation, V4+ = slashing
- Why a new major version? A new `Redelegated` needs to be processed by consensus and handled by UI clients.

### No new storage structs

Redelegation is instant, no pending state to track. No new mappings means the storage layout is trivially compatible
with V2.

### `initializeV3()`

Minimal. No new storage to initialize.

### New function

```solidity
function redelegate(
    address fromValidator,
    address toValidator,
    uint256 amount
) external whenNotPaused
```

**Checks:**

- `fromValidator` is active (via `ensureValidatorActive`)
- `toValidator` is active (via `ensureValidatorActive`)
- `fromValidator != toValidator`
- `amount > 0`
- `delegations[fromValidator][msg.sender] >= amount`
- `amount >= minDelegateAmount` (consistent with `delegate()`)
- Remaining source delegation is either 0 or `>= minDelegateAmount` (no dust positions)

**Note on exited validators:** `ensureValidatorActive` rejects exited validators. A delegator whose validator has exited
cannot redelegate, they must use the normal undelegate/claim flow. This is intentional: exited validator delegations are
frozen pending withdrawal. I think this is acceptable because validator exits are rare. We could implement re-delegation
after validator exit but I think it's not worth the complexity, especially because it likely makes implementing slashing
in the contract more difficult later.

**State changes:**

1. `delegations[fromValidator][msg.sender] -= amount`
2. `validators[fromValidator].delegatedAmount -= amount`
3. `delegations[toValidator][msg.sender] += amount`
4. `validators[toValidator].delegatedAmount += amount`
5. `activeStake` unchanged (decrements and increments cancel out)
6. Emit `Redelegated(msg.sender, fromValidator, toValidator, amount)`

### New event

```solidity
event Redelegated(
    address indexed delegator,
    address indexed fromValidator,
    address indexed toValidator,
    uint256 amount
);
```

### View functions

None needed beyond existing `delegations` getter.

### Note on interaction with slashing

If we implemented the slashing accounting in the contract (and not in rust) we will likely move to share based
accounting in the stake table contract. In a share-based system, delegations are tracked as proportional shares of a
validator's pool rather than absolute token amounts, allowing slashing to reduce all delegators' balances
proportionally. Shares are unnecessary without slashing. If we implement slashing later via in contract accounting, both
undelegation and redelegation can migrate to share-based together. Therefore the re-delegation change described here
should not have a major impact on the difficulty of implementing slashing later.

## 3. Rust Code Changes

### No new consensus version

Strict deployment sequence (Section 6) ensures all nodes support the event before the contract emits it. No protocol
version bump required.

### Files to change

**`types/src/v0/v0_3/stake_table.rs`**

- Add `Redelegate` variant to `StakeTableEvent`

**`types/src/v0/impls/stake_table.rs`**

- Rust bindings must be regenerated with the V3 ABI (`just gen-bindings`). This produces a new `StakeTableV3Events` enum
  containing the `Redelegated` variant. Without this, `decode_raw_log` silently drops the event.

- Add `TryFrom<StakeTableV3Events>` impl (or extend the existing V2 impl): map `Redelegated` filter log to
  `StakeTableEvent::Redelegate { delegator, from_validator, to_validator, amount }`

- `apply_event`: handle `Redelegate` as an atomic move:
  - Subtract `amount` from source validator's delegated stake and delegator's delegation
  - Add `amount` to destination validator's delegated stake and delegator's delegation
  - Pattern matches existing `Delegate` and `Undelegate` handlers

- Event filter: add `Redelegated::SIGNATURE` to the filter topic list

- `Debug` impl: add debug arm for `Redelegate`

## 4. Staking CLI Changes

**File:** `staking-cli/src/lib.rs`

- Add `Redelegate { from_validator, to_validator, amount }` command variant
- Implement: call `stake_table.redelegate(from_validator, to_validator, amount)`
- Add event display formatting for `Redelegated`

## 5. Delegation UI Changes

**Repo:** `espresso-block-explorer` (`packages/espresso-block-explorer-components`)

### Contract interface

Add `redelegate` to the StakeTable contract interface and ABI. Follows the existing `delegate`/`undelegate` pattern.

**Files:**

- `contracts/stake_table_v2/stake_table_v2_interface.ts` — extend V2 writeable interface with
  `redelegate(fromValidator, toValidator, amount)` (mirrors Solidity inheritance: `StakeTableV3 is StakeTableV2`)
- `contracts/stake_table_v2/stake_table_v2_abi.ts` — add `redelegate` function and `Redelegated` event ABI entries

### State machine

Add new states to `ValidatorSelectionContext` (`validator_selection_context.tsx`):

- `ValidatorConfirmedRedelegate` — user chose "Redelegate" from manage stake. Carries source `validatorAddress`. Shows
  amount input with "Max" button (auto-fills full delegation to avoid dust).
- `ValidatorConfirmedRedelegateSelectDest` — user entered amount, now picking destination. Carries source
  `validatorAddress` + `amount`.
- `ValidatorConfirmedRedelegateConfirm` — user selected destination, ready to sign. Carries source `validatorAddress` +
  destination address + `amount`.

All three extend `ValidatorSelectionWithConfirmation`. Back button navigates to the previous state in the sequence.

**Flow:** enter amount -> select destination -> confirm & sign.

### Components

**Modified:**

- `manage_stake_content.tsx` — add "Redelegate" button alongside "Delegate More" and "Undelegate". Pushes
  `ValidatorConfirmedRedelegate` state. Disabled when validator is exited.
- `staking_modal_validator_confirmed_content.tsx` — add routing for the three new states to their respective content
  components.

**New:**

- `redelegation_content.tsx` — amount input (reuse existing staking amount input pattern). "Next" button pushes to
  `ValidatorConfirmedRedelegateSelectDest`.
- `redelegation_select_dest_content.tsx` — validator picker filtered to active validators, excluding the source
  validator. Selecting a destination pushes to `ValidatorConfirmedRedelegateConfirm`.
- `redelegation_confirm_content.tsx` — confirmation screen showing source validator, destination validator, and amount.
  "Confirm" button triggers `performRedelegation`.

### Transaction flow

- New `perform_redelegation_context.tsx` — async generator context following the `performDelegation` /
  `performUndelegation` pattern.
- Calls `stakeTableContract.redelegate(fromValidator, toValidator, amount)`.
- On success, triggers L1 data refresh via `SetL1RefreshTimestampContext` (existing pattern).
- On wallet rejection (user cancels signing), return to confirm screen without error banner.
- No new data fetching needed — existing `WalletSnapshot` tracks delegations per validator.

### Event display

`Redelegated` events should appear in the delegation history / activity feed alongside `Delegated` and `Undelegated`
events. Display: "Redelegated {amount} from {sourceValidator} to {destValidator}".

## 6. Upgrade Strategy

V3 contract deployment with operational coordination:

1. Release new sequencer binary with `Redelegate` event support (dormant - no contract emits it yet)
2. All validators upgrade to new binary
3. Verify all (or almost all) validators are running the new binary (via metrics API endpoints or operator coordination)
4. Deploy `StakeTableV3`.

Governance controls timing.

**Rate limiting:** Redelegations will be quite cheap but do incur storage read/write costs. The cost will be similar to
delegations so we don't need extra rate limiting. The minimum amount enforcement helps to prevent creating too many
delegations with very low Esp token amounts.

## 7. Forward Compatibility with Slashing

When slashing lands:

- `redelegate` gains escrow: creates `Redelegation{shares, unlocksAt, shareValueAtRedelegation, toValidator}`
- New `claimRedelegation` function added
- `Redelegated` event gains escrow fields (or new `RedelegatedV2` event)
- Contract interface changes, but slashing is a major overhaul anyway
- Sequencer `apply_event` will need new event handling: with escrow, stake is removed from source immediately but only
  added to destination on claim. V4 will add new event(s) and updated `apply_event` handlers, similar to how
  `UndelegateV2` was added alongside `Undelegate`.

The current design does not make slashing harder to add:

- No escrow state to migrate (V3 has none; V4 adds it fresh)
- Event handling in Rust is additive (new event variant or updated mapping)
- The V3 `Redelegate` apply_event handler will be replaced, not extended
- If slashing uses share-based accounting (see Section 2 note), shares are added to both undelegation and redelegation
  together -- no partial retrofit needed

## 8. Test Plan

### Requirements

| ID                          | Description                                                                          |
| --------------------------- | ------------------------------------------------------------------------------------ |
| REQ:contract-redelegate     | Delegator can atomically move stake between validators                               |
| REQ:contract-balances       | Source delegation decreases, dest increases by exact amount                          |
| REQ:contract-active-stake   | `activeStake` unchanged after redelegation                                           |
| REQ:contract-event          | `Redelegated` event emitted with correct parameters                                  |
| REQ:rust-apply-event        | Sequencer processes `Redelegated` as atomic stake move                               |
| REQ:rust-hash-consistency   | Stake table hash reflects redelegation correctly                                     |
| REQ:cli-redelegate          | staking-cli has redelegate command that sends correct calldata                       |
| REQ:upgrade-deploy-sequence | Nodes must support event before contract deploy                                      |
| REQ:ui-redelegate-flow      | User can redelegate via manage stake -> redelegate -> select dest -> confirm -> sign |
| REQ:ui-redelegate-balances  | After redelegation, UI reflects updated delegation balances for both validators      |

### Requirement Tests

| Test                           | Requirement                |
| ------------------------------ | -------------------------- |
| TEST:contract-redelegate-ok    | REQ:contract-redelegate    |
| TEST:contract-balances-ok      | REQ:contract-balances      |
| TEST:contract-active-stake-ok  | REQ:contract-active-stake  |
| TEST:contract-event-ok         | REQ:contract-event         |
| TEST:rust-apply-event-ok       | REQ:rust-apply-event       |
| TEST:rust-hash-consistency-ok  | REQ:rust-hash-consistency  |
| TEST:cli-redelegate-ok         | REQ:cli-redelegate         |
| TEST:ui-redelegate-flow-ok     | REQ:ui-redelegate-flow     |
| TEST:ui-redelegate-balances-ok | REQ:ui-redelegate-balances |

### Edge Cases

| ID                                     | Description                                                                    |
| -------------------------------------- | ------------------------------------------------------------------------------ |
| EDGE:contract-same-validator           | Redelegate to same validator -- must revert                                    |
| EDGE:contract-zero-amount              | Redelegate with zero amount -- must revert                                     |
| EDGE:contract-insufficient-balance     | Amount exceeds delegation -- must revert                                       |
| EDGE:contract-inactive-source          | Source validator not active -- must revert                                     |
| EDGE:contract-inactive-dest            | Destination validator not active -- must revert                                |
| EDGE:contract-paused                   | Redelegate while contract paused -- must revert                                |
| EDGE:contract-full-redelegate          | Move entire delegation (delegation becomes 0) -- must succeed                  |
| EDGE:contract-no-delegation            | Sender has zero delegation to source validator -- must revert                  |
| EDGE:contract-dust-remaining           | Partial redelegate leaves source below `minDelegateAmount` -- must revert      |
| EDGE:contract-below-min-amount         | Amount below `minDelegateAmount` -- must revert                                |
| EDGE:contract-existing-dest-delegation | Redelegate to validator where sender already has a delegation -- additive      |
| EDGE:contract-during-undelegation      | Redelegate remaining delegation while undelegation is pending from same source |
| EDGE:contract-sequential-redelegations | A->B then B->C in same block                                                   |
| EDGE:rust-unknown-validator            | `Redelegated` event references unknown validator                               |
| EDGE:rust-insufficient-stake           | Amount exceeds tracked stake                                                   |

### UI Edge Cases

| ID                                | Description                                                                                            |
| --------------------------------- | ------------------------------------------------------------------------------------------------------ |
| EDGE:ui-redelegate-button-exited  | Redelegate button disabled/hidden when source validator is exited                                      |
| EDGE:ui-redelegate-no-delegation  | Redelegate button not shown when user has no delegation to validator                                   |
| EDGE:ui-dest-excludes-source      | Destination validator picker excludes the source validator                                             |
| EDGE:ui-amount-exceeds-delegation | Amount input rejects values exceeding current delegation                                               |
| EDGE:ui-amount-below-min          | Amount input rejects values below `minDelegateAmount`                                                  |
| EDGE:ui-dust-remaining            | Amount input rejects values that would leave source below `minDelegateAmount` (unless full redelegate) |
| EDGE:ui-dest-must-be-active       | Destination picker only shows active validators                                                        |
| EDGE:ui-tx-failure                | Transaction revert shows error state, does not update balances                                         |
| EDGE:ui-wallet-rejection          | User rejects wallet signing, UI returns to confirm state without error banner                          |
| EDGE:ui-additive-dest-balance     | Redelegating to validator with existing delegation shows updated (additive) balance                    |

### Withdrawal Path Exclusion

These tests verify that tokens cannot be double-spent across different withdrawal paths (redelegate, undelegate/claim,
claimValidatorExit). Each delegation can only exit through one path at a time.

| ID                                             | Description                                                                                   |
| ---------------------------------------------- | --------------------------------------------------------------------------------------------- |
| EDGE:no-claim-exit-after-full-redelegate       | Redelegate full amount, `claimValidatorExit` reverts `NothingToWithdraw`                      |
| EDGE:no-redelegate-after-full-undelegate       | Undelegate full amount, redelegate reverts (zero delegation remaining)                        |
| EDGE:redelegate-partial-then-claim-withdrawal  | Redelegate part, undelegate remainder, claim withdrawal -- amounts correct, no double-spend   |
| EDGE:claim-exit-partial-after-redelegate       | Redelegate part, validator exits, `claimValidatorExit` returns only the remainder             |
| EDGE:no-double-claim-exit                      | `claimValidatorExit` once succeeds, second call reverts `NothingToWithdraw`                   |
| EDGE:claim-withdrawal-then-claim-exit          | Undelegate part, validator exits, claim both paths -- each returns correct amount, no overlap |
| EDGE:no-undelegate-after-validator-exit        | Validator exits, `undelegate` reverts `ValidatorAlreadyExited`                                |
| EDGE:undelegate-then-redelegate-remainder      | Undelegate part, redelegate remainder, claim pending withdrawal -- amounts correct            |
| EDGE:redelegate-to-dest-then-dest-exits        | Redelegate A->B, B exits, `claimValidatorExit(B)` returns redelegated amount                  |
| EDGE:full-redelegate-then-undelegate-from-dest | Redelegate A->B full amount, undelegate from B, claim -- tokens exit normally via dest        |

### Edge Case Tests

| Test                                      | Edge Case                              |
| ----------------------------------------- | -------------------------------------- |
| TEST:contract-same-validator-fails        | EDGE:contract-same-validator           |
| TEST:contract-zero-amount-fails           | EDGE:contract-zero-amount              |
| TEST:contract-insufficient-balance-fails  | EDGE:contract-insufficient-balance     |
| TEST:contract-inactive-source-fails       | EDGE:contract-inactive-source          |
| TEST:contract-inactive-dest-fails         | EDGE:contract-inactive-dest            |
| TEST:contract-paused-fails                | EDGE:contract-paused                   |
| TEST:contract-full-redelegate-ok          | EDGE:contract-full-redelegate          |
| TEST:contract-no-delegation-fails         | EDGE:contract-no-delegation            |
| TEST:contract-dust-remaining-fails        | EDGE:contract-dust-remaining           |
| TEST:contract-below-min-amount-fails      | EDGE:contract-below-min-amount         |
| TEST:contract-existing-dest-delegation-ok | EDGE:contract-existing-dest-delegation |
| TEST:contract-during-undelegation-ok      | EDGE:contract-during-undelegation      |
| TEST:contract-sequential-redelegations-ok | EDGE:contract-sequential-redelegations |
| TEST:rust-unknown-validator-fails         | EDGE:rust-unknown-validator            |
| TEST:rust-insufficient-stake-fails        | EDGE:rust-insufficient-stake           |

### UI Edge Case Tests

| Test                                    | Edge Case                         |
| --------------------------------------- | --------------------------------- |
| TEST:ui-redelegate-button-exited-fails  | EDGE:ui-redelegate-button-exited  |
| TEST:ui-redelegate-no-delegation-fails  | EDGE:ui-redelegate-no-delegation  |
| TEST:ui-dest-excludes-source-ok         | EDGE:ui-dest-excludes-source      |
| TEST:ui-amount-exceeds-delegation-fails | EDGE:ui-amount-exceeds-delegation |
| TEST:ui-amount-below-min-fails          | EDGE:ui-amount-below-min          |
| TEST:ui-dust-remaining-fails            | EDGE:ui-dust-remaining            |
| TEST:ui-dest-must-be-active-ok          | EDGE:ui-dest-must-be-active       |
| TEST:ui-tx-failure-ok                   | EDGE:ui-tx-failure                |
| TEST:ui-wallet-rejection-ok             | EDGE:ui-wallet-rejection          |
| TEST:ui-additive-dest-balance-ok        | EDGE:ui-additive-dest-balance     |

### Withdrawal Path Exclusion Tests

| Test                                              | Edge Case                                      |
| ------------------------------------------------- | ---------------------------------------------- |
| TEST:no-claim-exit-after-full-redelegate          | EDGE:no-claim-exit-after-full-redelegate       |
| TEST:no-redelegate-after-full-undelegate          | EDGE:no-redelegate-after-full-undelegate       |
| TEST:redelegate-partial-then-claim-withdrawal-ok  | EDGE:redelegate-partial-then-claim-withdrawal  |
| TEST:claim-exit-partial-after-redelegate-ok       | EDGE:claim-exit-partial-after-redelegate       |
| TEST:no-double-claim-exit                         | EDGE:no-double-claim-exit                      |
| TEST:claim-withdrawal-then-claim-exit-ok          | EDGE:claim-withdrawal-then-claim-exit          |
| TEST:no-undelegate-after-validator-exit           | EDGE:no-undelegate-after-validator-exit        |
| TEST:undelegate-then-redelegate-remainder-ok      | EDGE:undelegate-then-redelegate-remainder      |
| TEST:redelegate-to-dest-then-dest-exits-ok        | EDGE:redelegate-to-dest-then-dest-exits        |
| TEST:full-redelegate-then-undelegate-from-dest-ok | EDGE:full-redelegate-then-undelegate-from-dest |

### Invariant Tests

Add `redelegate` as a fuzzing target to `StakeTableV2PropTestBase`:

- `redelegateOk(actorIndex, fromValIndex, toValIndex, amount)` — pick actor with existing delegation, two distinct
  active validators, bound amount to delegation balance
- `redelegateAny(actorIndex, fromValIndex, toValIndex, amount)` — raw fuzz input, expect reverts
- `trackRedelegate(actor, fromVal, toVal, amount)` — update delegator sets per validator and per-delegator balance ghost
  state (`actors.trackedFunds` is unchanged since delegated total stays the same); `totalDelegated`, `activeStake`, and
  `totalPendingWithdrawal` are unchanged since tokens move between validators

Existing invariants automatically cover redelegation once the target is added:

| Invariant                                            | What it catches                                               |
| ---------------------------------------------------- | ------------------------------------------------------------- |
| `invariant_ContractBalanceMatchesTrackedDelegations` | Redelegate creating/destroying tokens                         |
| `invariant_activeStakeMatchesTracked`                | Redelegate incorrectly changing `activeStake`                 |
| `assertActorsRecoveredFunds`                         | Tokens stuck or duplicated after redelegate + full withdrawal |
| `assertValidatorDelegatedAmountSum`                  | `delegatedAmount` drifting from sum of individual delegations |

### Integration Tests

| Test                         | Description                                                                                                                                  |
| ---------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| TEST:e2e-redelegate-pipeline | Redelegation event emitted on L1, fetched by sequencer, applied to stake table, reflected in active validator when new stake table is active |
