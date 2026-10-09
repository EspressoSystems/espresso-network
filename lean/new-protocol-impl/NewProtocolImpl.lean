module

public import NewProtocolImpl.Decide
public import NewProtocolImpl.Machine
public import NewProtocolImpl.Steps
public import NewProtocolImpl.Conformance
public import NewProtocolImpl.WitnessKit
public import NewProtocolImpl.FourNodes
public import NewProtocolImpl.FiveNodes
public import NewProtocolImpl.EpochWitness
public import NewProtocolImpl.LongEpochWitness
public import NewProtocolImpl.RevoteWitness
public import NewProtocolImpl.SplitWitness
public import NewProtocolImpl.LateCertWitness
public import NewProtocolImpl.ByzantineWitness
public import NewProtocolImpl.MinorityLockWitness
public import NewProtocolImpl.Checks

/-!
# A reference implementation

A machine that meets `NewProtocolSpec`, and the proof that it does.

* `NewProtocolSpec.Lists` (in the specification package): a history as finite
  lists, each proved to hold exactly what the specification's predicate names
* `NewProtocolImpl.Decide`: `Owed` over those lists, and so decidable; more output
  never makes an obligation owed
* `NewProtocolImpl.Machine`: the machine, whose state is its history
* `NewProtocolImpl.Steps`: extending a `ProtocolHistory` by a step
* `NewProtocolImpl.Conformance`: the machine's histories obey `ProtocolHistory`
  (`historyOf_protocol`), nothing is owed after any step (`historyOf_settled`), and
  a network of machines is `Prompt` for every `δ` (`prompt_of_machine`)
* `NewProtocolImpl.WitnessKit`: what the witnesses below share, over any schedule
  of inputs and step times: traces, histories, and delivery bounds
* `NewProtocolImpl.FourNodes`: headers, the anchor, a single-epoch configuration,
  and four nodes, one faulty, that several witnesses share
* `NewProtocolImpl.FiveNodes`: five nodes, one faulty, epochs of one block, and
  committees that change with the epoch, which the epoch witnesses share
* `NewProtocolImpl.EpochWitness`: `FiveNodes`'s nodes, a new epoch with every
  block; the premises hold together across the epoch changes
  (`EpochWitness.premises_met`), and at every boundary the leader sends its re-vote
  request and the next epoch's first block in the same view, with no timeout
  (`EpochWitness.epochs_change`)
* `NewProtocolImpl.LongEpochWitness`: the same nodes with two blocks to an epoch,
  so a node outside a committee follows the epoch's views on certificates alone and
  never locks on its first block (`LongEpochWitness.two_blocks`); the premises hold
  together (`LongEpochWitness.premises_met`)
* `NewProtocolImpl.RevoteWitness`: `EpochWitness`'s nodes, with every re-vote
  request arriving before the epoch change, so the outgoing committee answers it
  and the block gets a second `Cert2`; the view after it times out with a lock on
  the re-vote's `Cert1` (`RevoteWitness.revote_answered`), and the premises hold
  together (`RevoteWitness.premises_met`)
* `NewProtocolImpl.SplitWitness`: `FiveNodes`'s nodes with GST at the first timer.
  Before GST the first epoch change reaches only some nodes, so the first timeout
  votes name two epochs and form no certificate; after GST the timer's second round
  does (`SplitWitness.split_boundary`), and every later boundary is crossed as in
  `EpochWitness` (`SplitWitness.direct_after_gst`); the premises hold together
  (`SplitWitness.premises_met`)
* `NewProtocolImpl.LateCertWitness`: `FiveNodes`'s nodes with GST at the first
  timer. The first epoch's last block gets its `Cert1` only after every node timed
  its view out, so nobody votes2 at the block's view; the epoch change carries the
  re-vote's `Cert2`, and the next epoch's first block names the block's own `Cert1`
  behind a timeout certificate (`LateCertWitness.late_cert`); the premises hold
  together (`LateCertWitness.premises_met`)
* `NewProtocolImpl.ByzantineWitness`: `FourNodes`'s nodes with rotating
  leaders, the faulty one proposing different blocks to two honest nodes; no
  `Cert1` forms, the slow third node answers the one-honest indication before its
  own timer, and the next honest leader builds behind the timeout certificate
  (`ByzantineWitness.equivocation`). The run comes with one epoch or with three
  blocks to an epoch; in the second the faulty leader's view follows each epoch's
  last block, the timeout certificate is of the new epoch, and the next leader
  opens it, with no re-vote (`ByzantineWitness.boundary`). There `d` is honest
  in the first epoch, where it votes as a member, and faulty in every later one
  (`ByzantineWitness.honest_once`). The premises hold together in both
  (`ByzantineWitness.premises_met`)
* `NewProtocolImpl.MinorityLockWitness`: `FourNodes`'s nodes with GST at the
  first timer. One node gets the first block's payload late, so it times out on an
  older lock than the timeout certificate names, and locks on that one only after
  GST (`MinorityLockWitness.minority_lock`); the premises hold together
  (`MinorityLockWitness.premises_met`)
* `NewProtocolImpl.Checks`: the axioms those rest on

The machine is eager and slow: after each input it takes every obligation the
history holds, rescanning the history to find them. It shows the specification
can be met, and it is executable, so recorded traces can be replayed against it.
Nothing here binds another implementation.

`historyOf_protocol` takes one premise about the environment: validity reports
are truthful, since consensus cannot judge a block itself.
-/
