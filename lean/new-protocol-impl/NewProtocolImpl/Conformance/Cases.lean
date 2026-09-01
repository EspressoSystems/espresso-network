module

public import NewProtocolImpl.Conformance.Fields

/-!
# What a round does, in one equation

For each round: either it does nothing, or its guards hold and it produces
exactly one state and one output list. Everything the action obligations need
about a round is read off these.

Each is stated over a *variable* result `r`, with `round … = r` as a hypothesis,
rather than about `round …` directly. That keeps the state and output terms out
of the goal while the guards are taken apart: the proofs are `split` on the
guards, small term-level steps for the guards themselves, and `rfl`.
-/

@[expose] public section

set_option linter.unusedSectionVars false

open Std (TreeMap TreeSet)

namespace NewProtocol
namespace Impl

/-! ## Reading a guard backwards

A round's guard is a disjunction of the reasons not to act, so what a round that
acted knows is a negated disjunction. These four turn the pieces of one into the
positive facts the obligations are stated with, without a simp set.
-/

theorem bool_true_of_not_not {b : Bool} (h : ¬ ¬ b = true) : b = true :=
  Decidable.of_not_not h

theorem bool_false_of_not {b : Bool} (h : ¬ b = true) : b = false := by
  cases b with
  | false => rfl
  | true => exact absurd rfl h

theorem eq_none_of_not_isSome {α : Type} {o : Option α} (h : ¬ o.isSome = true) : o = none := by
  cases o with
  | none => rfl
  | some x => exact absurd rfl h

variable {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
variable {settled : TreeSet ViewNumber} {v : ViewNumber} {s : State}
variable {r : State × List Output}

/-! ## The lock -/

theorem advanceLock_cases (h : advanceLock s = r) :
    r = (s, [])
      ∨ ∃ c, s.bestLock = some c ∧ s.lockBelow c.view = true
          ∧ r = ({ s with lockedCert := some c,
                          currentView := max s.currentView (c.view + 1) },
                 [Output.send (.cert1 c)]) := by
  unfold advanceLock at h
  split at h
  · rename_i c hc
    split at h
    · exact Or.inr ⟨c, hc, ‹s.lockBelow c.view = true›, h.symm⟩
    · exact Or.inl h.symm
  · exact Or.inl h.symm

/-! ## Vote1s -/

theorem tryVote1_cases (h : tryVote1 node v s = r) :
    r = (s, [])
      ∨ ∃ p share, s.admitted.get? v = some p ∧ s.vidShares.get? v = some share
          ∧ s.timeoutView < v ∧ s.barredView < v ∧ v ∉ s.voted1Views
          ∧ s.validated.get? v = some (blockHash p) ∧ s.parentLinked p = true
          ∧ safeToExtend s.lockedCert p = true
          ∧ r = ({ s with voted1Views := s.voted1Views.insert v,
                          vote1Branches := s.vote1Branches.insert v p.parentCert.view },
                 [Output.send (.vote1 ⟨⟨blockHash p, p.epoch⟩, v, node⟩),
                  Output.send (.vidShare share)]) := by
  unfold tryVote1 at h
  split at h
  · exact Or.inl h.symm
  · rename_i hg
    have hto : s.timeoutView < v := Nat.lt_of_not_le fun hc => hg (Or.inl hc)
    have hbar : s.barredView < v := Nat.lt_of_not_le fun hc => hg (Or.inr (Or.inl hc))
    have hvoted : v ∉ s.voted1Views := fun hc =>
      hg (Or.inr (Or.inr (contains_iff_mem.mpr hc)))
    split at h
    · rename_i p share hp hs
      split at h
      · rename_i hj
        exact Or.inr ⟨p, share, hp, hs, hto, hbar, hvoted, hj.1, hj.2.1, hj.2.2, h.symm⟩
      · exact Or.inl h.symm
    · exact Or.inl h.symm

/-! ## Vote2s -/

theorem tryVote2_cases (h : tryVote2 cfg node v s = r) :
    r = (s, [])
      ∨ ∃ p c, s.admitted.get? v = some p ∧ s.lockable v = some c
          ∧ s.barredView < v ∧ v ∉ s.voted2Views ∧ s.cert2s.get? v = none
          ∧ v ∉ s.decidedViews ∧ s.aboveFloor cfg v = true ∧ s.vote1Skipped v = false
          ∧ s.lockBelow v = false
          ∧ r = ({ s with voted2Views := s.voted2Views.insert v },
                 [Output.send (.vote2 ⟨⟨blockHash p, p.epoch⟩, v, node⟩)]) := by
  unfold tryVote2 at h
  split at h
  · exact Or.inl h.symm
  · rename_i hg
    have hbar : s.barredView < v := Nat.lt_of_not_le fun hc => hg (Or.inl hc)
    have hvoted : v ∉ s.voted2Views := fun hc =>
      hg (Or.inr (Or.inl (contains_iff_mem.mpr hc)))
    have hc2 : s.cert2s.get? v = none :=
      eq_none_of_not_isSome fun hc => hg (Or.inr (Or.inr (Or.inl hc)))
    have hdec : v ∉ s.decidedViews := fun hc =>
      hg (Or.inr (Or.inr (Or.inr (Or.inl (contains_iff_mem.mpr hc)))))
    have hfl : s.aboveFloor cfg v = true :=
      bool_true_of_not_not fun hc => hg (Or.inr (Or.inr (Or.inr (Or.inr (Or.inl hc)))))
    have haround : s.vote1Skipped v = false :=
      bool_false_of_not fun hc =>
        hg (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inl hc))))))
    have hlock : s.lockBelow v = false :=
      bool_false_of_not fun hc =>
        hg (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr hc))))))
    split at h
    · rename_i c hc
      split at h
      · rename_i p hp
        exact Or.inr ⟨p, c, hp, hc, hbar, hvoted, hc2, hdec, hfl, haround, hlock, h.symm⟩
      · exact Or.inl h.symm
    · exact Or.inl h.symm

/-! ## Proposals -/

theorem tryPropose_cases (h : tryPropose cfg leader node v s = r) :
    r = (s, [])
      ∨ ∃ p, ((s.timeoutCandidate cfg v).or (s.normalCandidate cfg v)) = some p
          ∧ s.timeoutView < v ∧ s.barredView < v ∧ v ∉ s.proposedViews
          ∧ leader v = some node
          ∧ r = ({ s with proposedViews := s.proposedViews.insert v },
                 [Output.send (.proposal p)]) := by
  unfold tryPropose at h
  split at h
  · exact Or.inl h.symm
  · rename_i hg
    have hto : s.timeoutView < v := Nat.lt_of_not_le fun hc => hg (Or.inl hc)
    have hbar : s.barredView < v := Nat.lt_of_not_le fun hc => hg (Or.inr (Or.inl hc))
    have hprop : v ∉ s.proposedViews := fun hc =>
      hg (Or.inr (Or.inr (Or.inl (contains_iff_mem.mpr hc))))
    have hlead : leader v = some node :=
      Decidable.of_not_not fun hc => hg (Or.inr (Or.inr (Or.inr hc)))
    split at h
    · rename_i p hp
      exact Or.inr ⟨p, hp, hto, hbar, hprop, hlead, h.symm⟩
    · exact Or.inl h.symm

/-! ## The lock, read out

Two lemmas the vote2 needs: what licenses a lock at a view, and what a
failed `lockBelow` test says.
-/

/-- A lockable certificate is one over the admitted, reconstructed block of its view. -/
theorem lockable_spec {c : Cert1} (h : s.lockable v = some c) :
    s.cert1s.get? v = some c
      ∧ ∃ p, s.admitted.get? v = some p ∧ c.data.blockHash = blockHash p
          ∧ c.data.epoch = p.epoch
          ∧ (v, p.payloadCommit) ∈ s.blocksReconstructed := by
  unfold State.lockable at h
  split at h
  · rename_i c' p hc hp
    split at h
    · rename_i hcond
      obtain ⟨hbh, hep, hrec⟩ := hcond
      cases h
      refine ⟨hc, p, hp, hbh, hep, ?_⟩
      unfold State.reconstructed at hrec
      exact contains_iff_mem.mp hrec
    · exact absurd h (by simp)
  · exact absurd h (by simp)

/--
The parent link, read out: unless the parent is genesis, we hold it, the parent
certificate names it, and its payload was reconstructed.
-/
theorem parentLinked_spec {p : Proposal} (h : s.parentLinked p = true) :
    p.parentCert.view ≠ ViewNumber.genesis →
      ∃ parent, s.proposals.get? p.parentCert.view = some parent
        ∧ p.parentCert.data.blockHash = blockHash parent
        ∧ (p.parentCert.view, parent.payloadCommit) ∈ s.blocksReconstructed := by
  intro hne
  unfold State.parentLinked at h
  rw [Bool.or_eq_true] at h
  rcases h with hgen | h
  · exact absurd (of_decide_eq_true (by simpa using hgen)) hne
  · split at h
    · rename_i parent hpar
      rw [Bool.and_eq_true] at h
      refine ⟨parent, hpar, of_decide_eq_true (by simpa using h.1), ?_⟩
      unfold State.reconstructed at h
      exact contains_iff_mem.mp h.2
    · exact absurd h (by simp)

/-- The lock has reached `v` exactly when the test that it sits below `v` fails. -/
theorem le_of_lockBelow_false (h : s.lockBelow v = false) :
    ∃ l, s.lockedCert = some l ∧ v ≤ l.view := by
  unfold State.lockBelow at h
  split at h
  · rename_i l hl
    exact ⟨l, hl, Nat.ge_of_not_lt (of_decide_eq_false h)⟩
  · exact absurd h (by simp)

/-! ## Deciding

The chain is existentially quantified with its defining equation alongside, so the
walk appears once rather than in every clause about the blocks decided.
-/

theorem tryDecide_cases (h : tryDecide cfg settled v s = r) :
    r = (s, [])
      ∨ ∃ p c1 c2 chain, s.proposals.get? v = some p ∧ s.cert1s.get? v = some c1
          ∧ s.cert2s.get? v = some c2 ∧ c2.data.blockHash = blockHash p
          ∧ v ∉ s.decidedViews ∧ floorOf cfg settled < v
          ∧ chain = s.decideChain settled (floorOf cfg settled) p
          ∧ r = ({ s with decidedViews :=
                     chain.foldl (fun d b => d.insert b.viewNumber) s.decidedViews },
                 [Output.decided chain c1 c2]) := by
  unfold tryDecide at h
  split at h
  · exact Or.inl h.symm
  · rename_i hg
    have hdec : v ∉ s.decidedViews := fun hc => hg (Or.inl (contains_iff_mem.mpr hc))
    have hfl : floorOf cfg settled < v := Decidable.of_not_not fun hc => hg (Or.inr hc)
    split at h
    · rename_i c2 c1 p h2 h1 hp
      split at h
      · rename_i hbh
        exact Or.inr ⟨p, c1, c2, _, hp, h1, h2, hbh, hdec, hfl, rfl, h.symm⟩
      · exact Or.inl h.symm
    · exact Or.inl h.symm

end Impl
end NewProtocol
