# Contract Tests

This directory contains test suites for the Espresso Sequencer smart contracts, focusing on the StakeTable contract.

## Running Tests

```bash
just contracts-test-forge      # Standard forge tests
just contracts-test-fuzz       # Fuzz tests
just contracts-test-invariant  # Invariant tests
just contracts-test-echidna    # Echidna property tests
```

## Invariant Testing

The invariant tests explore the state space of the StakeTable contract through systematic transaction execution. The
framework has two main goals:

1. Test that the balance accounting is correct as the state space evolves.
2. Ensure that contract safety is preserved as the state space evolves.
3. Ensure that each participant can withdraw all their funds in the end. Note that this is currently only done for the
   foundry invariant test, because it's unclear how to achieve it with echidna.

The tests track detailed statistics about function calls, reverts, and state changes to provide insight into test
coverage and state space exploration.

Key challenges are:

1. Finding good invariant to track. More should be added.
1. Evolving the state space: a lot of extra tracking is needed to have a reasonable fraction of non-reverting
   transactions.

### Shared Base Contract

Both Foundry (`StakeTableV2.invariant.t.sol`) and Echidna (`StakeTableV2.echidna.sol`) tests inherit from the same base
contract (`StakeTableV2PropTestBase.sol`) to minimize code duplications.

It was difficult to choose one of the two frameworks. Using both gives us extra coverage.

The base contract manages test actors, tracks system state, and provides helper functions for both testing frameworks.
Statistics are displayed through the `InvariantStats` utility contract.

### Key Invariants

- Global accounting accuracy: Contract balance equals tracked delegations plus pending withdrawals.
- Individual accounting accuracy: The tokens owned by each actor are always accounted for.
