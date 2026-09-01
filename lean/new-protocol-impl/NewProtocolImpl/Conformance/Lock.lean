module

public import NewProtocolImpl.Conformance.Decide

/-!
# The lock

`SafetySpec.lockJustified`, the one obligation `Impl.advanceLock` answers for.

The lock moves to the highest certificate the state licenses, which is a fold
over the admitted views; so the first thing to establish is that whatever the
fold returns is one of the certificates it looked at
(`Impl.bestLock_spec`). After that the obligation is the usual transfer: the
certificate is over the admitted, reconstructed block of its view — and the
specification asks only that the block be *held*, so the machine is stricter
here than it needs to be — and the lock it displaced was strictly below it.
-/

@[expose] public section

set_option linter.unusedSectionVars false

open Std (TreeMap TreeSet)

namespace NewProtocol
namespace Impl

variable {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
variable {settled : TreeSet ViewNumber} {t : State}

/-! ## The best lock is a lock

`better` keeps one of the two certificates it is given, so a fold of it over the
lockable certificates returns one of them.
-/

theorem better_spec (a : Option Cert1) (c : Cert1) : better a c = c ∨ a = some (better a c) := by
  unfold better
  split
  · rename_i b
    split
    · exact Or.inl rfl
    · exact Or.inr rfl
  · exact Or.inl rfl

theorem foldl_lockable (s : State) :
    ∀ (vs : List ViewNumber) (acc : Option Cert1) (c : Cert1),
      vs.foldl (fun best v =>
          match s.lockable v with
          | some c => some (better best c)
          | none => best) acc = some c →
        acc = some c ∨ ∃ v ∈ vs, s.lockable v = some c := by
  intro vs
  induction vs with
  | nil => intro acc c h; exact Or.inl h
  | cons v vs ih =>
    intro acc c h
    cases hl : s.lockable v with
    | none =>
      rw [List.foldl_cons, hl] at h
      rcases ih acc c h with hacc | ⟨w, hw, hlw⟩
      · exact Or.inl hacc
      · exact Or.inr ⟨w, List.mem_cons_of_mem _ hw, hlw⟩
    | some c' =>
      rw [List.foldl_cons, hl] at h
      rcases ih (some (better acc c')) c h with hacc | ⟨w, hw, hlw⟩
      · -- the accumulator that survived is either `c'` or the one it came in with
        have hc : better acc c' = c := by simpa using hacc
        rcases better_spec acc c' with hb | hb
        · exact Or.inr ⟨v, List.mem_cons_self .., by rw [hl, hb.symm.trans hc]⟩
        · exact Or.inl (by rw [hb, hc])
      · exact Or.inr ⟨w, List.mem_cons_of_mem _ hw, hlw⟩

/-- The lock the state licenses is a certificate over an admitted, reconstructed block. -/
theorem bestLock_spec {s : State} {c : Cert1} (h : s.bestLock = some c) :
    ∃ v, s.lockable v = some c := by
  rcases foldl_lockable s s.admitted.keys none c h with hnone | ⟨v, -, hl⟩
  · exact absurd hnone (by simp)
  · exact ⟨v, hl⟩

/-! ## The obligations -/

/-- The lock at the end of a pass: unchanged, or moved to a certificate that justifies it. -/
theorem pass_lock (hwf : WF cfg t) :
    (st5 cfg leader node t).lockedCert = t.lockedCert
      ∨ ∃ c, (st5 cfg leader node t).lockedCert = some c
          ∧ (∀ old, t.lockedCert = some old → old.view < c.view)
          ∧ t.cert1s.get? c.view = some c
          ∧ ∃ p, t.admitted.get? c.view = some p ∧ blockHash p = c.data.blockHash
              ∧ (c.view, p.payloadCommit) ∈ t.blocksReconstructed := by
  obtain ⟨hf1, hle1, hwf1⟩ := st1_stage (cfg := cfg) hwf
  have h1lock : (st1 cfg t).lockedCert = t.lockedCert :=
    seq_proj State.lockedCert (fun f hf t' => by
      obtain ⟨v, -, rfl⟩ :=
        List.mem_map.mp (show f ∈ List.map (tryDecide cfg t.decidedViews) t.cert2s.keys from hf)
      exact tryDecide_lock t') t
  rcases advanceLock_cases (r := advanceLock (st1 cfg t)) rfl with heq | ⟨c, hbest, hbelow, heq⟩
  · refine Or.inl ?_
    rw [st5_lock, st2, heq]
    exact h1lock
  · refine Or.inr ⟨c, ?_, ?_, ?_⟩
    · rw [st5_lock, st2, heq]
    · intro old hold
      unfold State.lockBelow at hbelow
      rw [show (st1 cfg t).lockedCert = some old from h1lock.trans hold] at hbelow
      exact of_decide_eq_true hbelow
    · obtain ⟨v, hl⟩ := bestLock_spec hbest
      obtain ⟨hc1, p, hadm, hbh, -, hrec⟩ := lockable_spec hl
      have hcv : c.view = v := hwf1.cert1s _ _ hc1
      refine ⟨?_, p, ?_, hbh.symm, ?_⟩
      · rw [hcv, ← hf1.cert1s]; exact hc1
      · rw [hcv, ← hf1.admitted]; exact hadm
      · rw [hcv, ← hf1.blocksReconstructed]; exact hrec

end Impl
end NewProtocol
