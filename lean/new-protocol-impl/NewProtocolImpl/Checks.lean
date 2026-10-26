module

import NewProtocolImpl.Conformance
import NewProtocolImpl.EpochWitness
import NewProtocolImpl.LongEpochWitness
import NewProtocolImpl.RevoteWitness
import NewProtocolImpl.SplitWitness
import NewProtocolImpl.LateCertWitness
import NewProtocolImpl.ByzantineWitness
import NewProtocolImpl.MinorityLockWitness

/-!
# Checks on the claims this package makes about itself

The machine's theorems rest on Lean's own axioms and nothing else: `sorryAx` would
appear here if any step of the argument were incomplete. Checked at build time,
since the specification's own checks cannot see this package.
-/

namespace NewProtocolImpl.Checks

/-- info: 'NewProtocolImpl.historyOf_protocol' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms historyOf_protocol

/-- info: 'NewProtocolImpl.historyOf_settled' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms historyOf_settled

/-- info: 'NewProtocolImpl.prompt_of_machine' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms prompt_of_machine

/--
info: 'NewProtocolImpl.EpochWitness.premises_met' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in #print axioms EpochWitness.premises_met

/--
info: 'NewProtocolImpl.EpochWitness.epochs_change' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in #print axioms EpochWitness.epochs_change

/-- info: 'NewProtocolImpl.EpochWitness.decides' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms EpochWitness.decides

/--
info: 'NewProtocolImpl.LongEpochWitness.premises_met' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in #print axioms LongEpochWitness.premises_met

/--
info: 'NewProtocolImpl.LongEpochWitness.two_blocks' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in #print axioms LongEpochWitness.two_blocks

/--
info: 'NewProtocolImpl.LongEpochWitness.decides' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in #print axioms LongEpochWitness.decides

/--
info: 'NewProtocolImpl.RevoteWitness.premises_met' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in #print axioms RevoteWitness.premises_met

/--
info: 'NewProtocolImpl.RevoteWitness.revote_answered' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in #print axioms RevoteWitness.revote_answered

/-- info: 'NewProtocolImpl.RevoteWitness.decides' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms RevoteWitness.decides

/--
info: 'NewProtocolImpl.SplitWitness.premises_met' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in #print axioms SplitWitness.premises_met

/--
info: 'NewProtocolImpl.SplitWitness.split_boundary' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in #print axioms SplitWitness.split_boundary

/--
info: 'NewProtocolImpl.SplitWitness.direct_after_gst' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in #print axioms SplitWitness.direct_after_gst

/-- info: 'NewProtocolImpl.SplitWitness.decides' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms SplitWitness.decides

/--
info: 'NewProtocolImpl.LateCertWitness.premises_met' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in #print axioms LateCertWitness.premises_met

/--
info: 'NewProtocolImpl.LateCertWitness.late_cert' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in #print axioms LateCertWitness.late_cert

/-- info: 'NewProtocolImpl.LateCertWitness.decides' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms LateCertWitness.decides

/--
info: 'NewProtocolImpl.ByzantineWitness.premises_met' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in #print axioms ByzantineWitness.premises_met

/--
info: 'NewProtocolImpl.ByzantineWitness.equivocation' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in #print axioms ByzantineWitness.equivocation

/--
info: 'NewProtocolImpl.ByzantineWitness.decides' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in #print axioms ByzantineWitness.decides

/-- info: 'NewProtocolImpl.ByzantineWitness.boundary' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ByzantineWitness.boundary

/--
info: 'NewProtocolImpl.ByzantineWitness.honest_once' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in #print axioms ByzantineWitness.honest_once

/--
info: 'NewProtocolImpl.MinorityLockWitness.premises_met' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in #print axioms MinorityLockWitness.premises_met

/--
info: 'NewProtocolImpl.MinorityLockWitness.minority_lock' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in #print axioms MinorityLockWitness.minority_lock

/--
info: 'NewProtocolImpl.MinorityLockWitness.decides' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in #print axioms MinorityLockWitness.decides

end NewProtocolImpl.Checks
