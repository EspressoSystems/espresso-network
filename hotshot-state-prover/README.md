# HotShot Light Client and State Prover

## [Runbook](RUNBOOK.md)

## Overview

### Light Client

The HotShot light client is an on-chain contract that tracks the latest state of the HotShot consensus protocol. The
light client maintains the following fields:

| Field             | Details                                  |
| ----------------- | ---------------------------------------- |
| `view_number`     | The latest view number of the consensus  |
| `block_height`    | The latest block height of the consensus |
| `block_comm_root` | Root commitment of the block Merkle tree |

The block Merkle tree accumulates all historical blocks up to `block_height`. Its root commitment serves as an
authenticated checkpoint for applications relying on HotShot consensus.

To update its state, the light client contract accepts a _state update proof_. This proof asserts that a quorum of
HotShot validators have signed the new state.

Additionally, the contract maintains the _stake table state_, which identifies the set of HotShot validators. The stake
table consists of:

1. A hash of all consensus public keys
2. A hash of all state signing keys
3. A hash of all stake weights
4. The threshold for the total weight required to update the contract state

### HotShot State Prover

The HotShot state prover is responsible for generating _state update proofs_ that the light client contract can verify.

The workflow for updating the light client state after a new HotShot block is finalized is:

1. **Validator Signing:** Each validator signs the new light client state after block finalization and sends their
   signature to a relay server.
2. **Signature Aggregation:** The relay server collects signatures from a quorum of validators and provides them to the
   state prover.
3. **Proof Generation:** The state prover generates a Plonk proof that:
   - All signers are present in the correct stake table
   - All signatures on the light client state are valid
   - The total stake weight of the signers exceeds the quorum threshold
4. **State Update:** The light client contract verifies the proof and updates its state upon successful verification.

### PoS Upgrade

With the PoS upgrade, the light client introduces new mechanics to handle dynamic stake tables across epochs:

- For each epoch, the light client state is not updated during the last 4 blocks. Instead, at the fifth-to-last
  block—called the _epoch root_—the light client state is updated to include the new stake table state for the upcoming
  epoch.

- Upon the quorum proposal for the epoch root, validators speculatively compute both the light client state and next
  epoch's stake table state. They sign these states and send their signatures to the leader (not the relay server).

- The leader additionally aggregates the signatures from a quorum of validators into the quorum certificate. The epoch
  root block is only finalized if its quorum certificate contains sufficient signatures.

- During the epoch transition, the state prover queries the epoch root's quorum certificate for the new states and their
  signatures, and generates a Plonk proof that can be verified by the light client contract.

These changes ensure that the stake table transitions are securely and efficiently reflected in the light client,
maintaining consensus integrity across epochs.

## Legacy Service

The prover can also run in a legacy mode for older contract versions. The application automatically detects the contract
version and runs the appropriate service. The legacy service implementation can be found in `src/legacy/service.rs`.
