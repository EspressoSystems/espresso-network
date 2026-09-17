module

public import NewProtocolSpec.Assumptions
public import NewProtocolSpec.Safety.Defs
public import NewProtocolSpec.Safety.Lemmas

/-!
# Protocol-level safety

`DecideSafety`, that the `Cert2`-certified blocks form a single chain, and
`decideSafety`, its proof for every network of honest nodes.
-/

@[expose] public section

namespace NewProtocol

variable (tree : BlockTable)

/--
**Decide safety (no forks)**: of any two `Cert2`-certified blocks, the one
certified in the earlier view is an ancestor of the other.

This is the property the protocol exists to provide. It implies that
all decided blocks lie on a single chain, and (`cert2_unique`) that at most one
block decides per view.

Stated over a `Network`, which is what gives the proof its hypothesis: that
the certificates come from nodes obeying `SafetySpec`. Every ingredient of the
argument is a per-node rule, and only a network connects those rules to
certificates.

Those twenty-one clauses are the whole of what is assumed of a node here, and
`Network` is built on them rather than on `StepSpec` so that this stays true as
the argument grows: a step of a node in this statement is not known to satisfy
any other rule, so no proof below can quietly come to depend on one. `leader`
is absent for the same reason — no clause of `SafetySpec` mentions it, so a fork
cannot be laid at the door of leader election.

The three conditions on `tree` are not decoration. `Ancestor` is defined by
following `tree`, so without them the statement is simply false: an empty tree
makes ancestry collapse to equality of hashes. `Resolves` is the one that
does the work — it says the tree contains the blocks the network actually
holds, which is what lets a chain be walked at all. `TreeCoherent` and
`CollisionFree` then make "same hash" mean "same block".

The argument, and what it rests on. Suppose a block is certified at a view
above `v` on a branch whose ancestry omits `v`. That branch is a chain of
`parentCert` links passing over `v`, so somewhere on it sits a proposal at some
`w > v` justified below `v` — a gap. Its `Cert1` is a quorum of vote1s at
`w`, and `Committee.intersect` gives an honest node in both that quorum and the
vote2 quorum at `v`. Such a node cannot exist, in either order it might have
acted:

* Vote1 at `w` first. By `vote1Records` it kept the branch it endorsed, and
  `vote2NotInSkippedView` then refuses the vote2 at `v` — the branch holds no
  block there.
* Vote2 at `v` first. By `vote2LockOrdered` its lock is at `v` or beyond, and
  `Vote1Justification.safeToExtend` refuses the vote1 at `w`: the gap
  proposal's parent sits below `v`, so it neither matches the lock nor is newer
  than it.

The node this runs on is chosen rather than taken from the intersection alone: it
is one that signed at `w` without being locked there, which exists because being
locked on a certificate means it was delivered after a quorum had signed it, and
that regress terminates (`Network.beforeWF`). Without that step the second case
has a hole — a node locked at `w` finds the proposal there safe on a commitment
match, without its parent being read at all.

Neither half suffices alone, and neither is a check on the branch being voted
for — both are checks against what the node has already done. That is why no
test made at the moment of admission can stand in for them: admission sees the
lock as it was, and both orders turn on the lock or the vote record moving
afterwards.

The timeout bar plays no part. Timing out endorses no branch, so it neither
licenses nor forbids either vote; an argument routed through it would also have
to explain nodes that never timed out at all.
-/
def DecideSafety {C : Committee} (N : Network cfg C) : Prop :=
  TreeCoherent tree → CollisionFree → Resolves tree N →
    ∀ c c', Network.ValidCert2 cfg N c → Network.ValidCert2 cfg N c' →
      c.view ≤ c'.view → Ancestor tree c.data.blockHash c'.data.blockHash

/--
Decide safety holds for every network of honest nodes.

`cert2_implies_cert1` turns the later commit into a `Cert1` over the
same block, and `cert2_ancestor` walks down to the earlier one.
-/
theorem decideSafety {C : Committee} (N : Network cfg C) (hcfg : ConfigCoherent cfg) :
    DecideSafety tree N := by
  intro _htc hcf hres c c' hc hc' hle
  obtain ⟨c1, hb1, hv1, hh1⟩ := cert2_implies_cert1 cfg N hcfg hc'
  have := cert2_ancestor tree N hcfg hcf hres hc c1.view.toNat c1 (Nat.le_refl _) hb1
    (by rw [hv1]; exact hle)
  rw [hh1] at this
  exact this

end NewProtocol
