module

public import NewProtocolImpl.Conformance.Settle

/-!
# The guards, read the other way

The safety half needed every guard to be *sound*: what the machine does, the
specification allows. Eagerness needs them *complete*: what the specification
says is owed, the machine does. Most of the guards are already equivalences —
`Impl.safeToExtend_iff`, `wellFormed_iff`, `vote1Skipped_iff` — and this file
supplies the three that were only ever needed in one direction, together with
what the vote2 turns on.

That last one is the reason the lock advance runs before the votes. A vote2
vote is barred while the lock sits below its view, and nothing in
`Vote2Enabled` mentions the lock: what makes the two agree is that a state
licensing a lock at `v` (`Impl.State.lockable`) makes `Impl.advanceLock`
move the lock to `v` or beyond, since it moves to the highest licensed
certificate (`Impl.le_bestLock`).
-/

@[expose] public section

set_option linter.unusedSectionVars false

open Std (TreeMap TreeSet)

namespace NewProtocol
namespace Impl

variable {cfg : Config} {s : State} {v : ViewNumber}

/-! ## Content shared by two states -/

/-- Two states that frame the same state frame each other. -/
theorem Frame.swap {t a b : State} (h1 : Frame t a) (h2 : Frame t b) : Frame a b where
  proposals := h2.proposals.trans h1.proposals.symm
  admitted := h2.admitted.trans h1.admitted.symm
  vidShares := h2.vidShares.trans h1.vidShares.symm
  validated := h2.validated.trans h1.validated.symm
  blocksReconstructed := h2.blocksReconstructed.trans h1.blocksReconstructed.symm
  headers := h2.headers.trans h1.headers.symm
  cert1s := h2.cert1s.trans h1.cert1s.symm
  cert2s := h2.cert2s.trans h1.cert2s.symm
  timeoutCerts := h2.timeoutCerts.trans h1.timeoutCerts.symm
  barredView := h2.barredView.trans h1.barredView.symm
  timeoutView := h2.timeoutView.trans h1.timeoutView.symm
  currentEpoch := h2.currentEpoch.trans h1.currentEpoch.symm

/-! ## Two tests the specification's clauses imply -/

/-- The parent link test passes whenever the specification's clause holds. -/
theorem parentLinked_of_spec {p : Proposal}
    (h : p.parentCert.view ≠ ViewNumber.genesis →
      ∃ parent, s.proposals.get? p.parentCert.view = some parent
        ∧ p.parentCert.data.blockHash = blockHash parent
        ∧ (p.parentCert.view, parent.payloadCommit) ∈ s.blocksReconstructed) :
    s.parentLinked p = true := by
  unfold State.parentLinked
  rw [Bool.or_eq_true]
  by_cases hg : p.parentCert.view = ViewNumber.genesis
  · exact Or.inl (by simp [hg])
  · refine Or.inr ?_
    obtain ⟨parent, hpar, hbh, hrec⟩ := h hg
    rw [hpar]
    dsimp only
    rw [Bool.and_eq_true]
    refine ⟨by simp [hbh], ?_⟩
    unfold State.reconstructed
    exact contains_iff_mem.mpr hrec

/-- A certificate over the held, reconstructed block of its view licenses a lock there. -/
theorem lockable_of_spec {c : Cert1} {p : Proposal} (hc : s.cert1s.get? v = some c)
    (hp : s.proposals.get? v = some p) (hbh : c.data.blockHash = blockHash p)
    (hep : c.data.epoch = p.epoch)
    (hrec : (v, p.payloadCommit) ∈ s.blocksReconstructed) : s.lockable v = some c := by
  unfold State.lockable
  rw [hc, hp]
  dsimp only
  rw [if_pos ⟨hbh, hep, by unfold State.reconstructed; exact contains_iff_mem.mpr hrec⟩]

/-! ## The lock reaches every view it may

The search for the lock is a fold, so both facts about it are folds: the
accumulator only ever climbs, and every licensed certificate enters it.
-/

/-- The step the lock search folds. -/
def lockStep (s : State) (best : Option Cert1) (v : ViewNumber) : Option Cert1 :=
  match s.lockable v with
  | some c => some (better best c)
  | none => best

theorem bestLock_eq (s : State) : s.bestLock = s.cert1s.keys.foldl (lockStep s) none := rfl

theorem le_better_right (a : Option Cert1) (c : Cert1) : c.view ≤ (better a c).view := by
  unfold better
  cases a with
  | none => exact Nat.le_refl _
  | some b =>
    dsimp only
    split
    · exact Nat.le_refl _
    · rename_i hlt; exact Nat.le_of_not_lt hlt

theorem le_better_left {a : Option Cert1} {b : Cert1} (h : a = some b) (c : Cert1) :
    b.view ≤ (better a c).view := by
  subst h
  unfold better
  dsimp only
  split
  · rename_i hlt; exact Nat.le_of_lt hlt
  · exact Nat.le_refl _

/-- The search never loses ground. -/
theorem foldl_lockStep_mono (s : State) : ∀ (l : List ViewNumber) (acc : Option Cert1) (b : Cert1),
    acc = some b → ∃ d, l.foldl (lockStep s) acc = some d ∧ b.view ≤ d.view
  | [], acc, b, h => ⟨b, h, Nat.le_refl _⟩
  | w :: l, acc, b, h => by
    show ∃ d, l.foldl (lockStep s) (lockStep s acc w) = some d ∧ b.view ≤ d.view
    unfold lockStep
    rcases hl : s.lockable w with _ | c
    · dsimp only
      exact foldl_lockStep_mono s l acc b h
    · dsimp only
      obtain ⟨d, hd, hle⟩ := foldl_lockStep_mono s l (some (better acc c)) (better acc c) rfl
      exact ⟨d, hd, Nat.le_trans (le_better_left h c) hle⟩

/-- Every licensed certificate on the scan is one the search ends at or above. -/
theorem foldl_lockStep_mem (s : State) : ∀ (l : List ViewNumber) (acc : Option Cert1)
    (v : ViewNumber), v ∈ l → ∀ c : Cert1, s.lockable v = some c →
      ∃ d, l.foldl (lockStep s) acc = some d ∧ c.view ≤ d.view
  | [], _, _, hv, _, _ => absurd hv (by simp)
  | w :: l, acc, v, hv, c, hc => by
    show ∃ d, l.foldl (lockStep s) (lockStep s acc w) = some d ∧ c.view ≤ d.view
    rcases List.mem_cons.mp hv with rfl | hv'
    · unfold lockStep
      rw [hc]
      obtain ⟨d, hd, hle⟩ := foldl_lockStep_mono s l (some (better acc c)) (better acc c) rfl
      exact ⟨d, hd, Nat.le_trans (le_better_right acc c) hle⟩
    · exact foldl_lockStep_mem s l (lockStep s acc w) v hv' c hc

/--
The lock the search settles on is at or above every view the state licenses.

The scan needs no membership hypothesis: a licensed view holds the certificate
it licenses, so it is one the search visits.
-/
theorem le_bestLock {c : Cert1} (h : s.lockable v = some c) :
    ∃ b, s.bestLock = some b ∧ c.view ≤ b.view := by
  rw [bestLock_eq]
  exact foldl_lockStep_mem s _ none v (mem_keys_of_get? (lockable_spec h).1) c h

/--
After the lock advance the lock has reached every view the state licensed.

Either the search's certificate is taken, or the lock already sat at or above it
— which is exactly what the test `Impl.advanceLock` makes says.
-/
theorem advanceLock_reached {c : Cert1} (h : s.lockable v = some c) :
    ∃ l, (advanceLock s).1.lockedCert = some l ∧ c.view ≤ l.view := by
  obtain ⟨b, hb, hle⟩ := le_bestLock h
  unfold advanceLock
  rw [hb]
  dsimp only
  split
  · exact ⟨b, rfl, hle⟩
  · rename_i hbelow
    obtain ⟨l, hl, hlb⟩ := le_of_lockBelow_false (Bool.eq_false_iff.mpr hbelow)
    exact ⟨l, hl, Nat.le_trans hle hlb⟩

/-! ## Owed decides do not move within a pass

The decide round is judged against the state the pass began at, so the field of
`Settled` has to be carried from the end of the pass back to there — which is
possible because a decide reads content, which the pass frames, and freshness,
which only ever grows.
-/

/--
A decide owed at the end of a pass was owed at its start.

The decided views may have grown in between, and that is the direction this
needs: a view undecided at the end was undecided at the start, and a floor that
held then held before, fewer decided views being a weaker bound.
-/
theorem decideEnabled_congr {a b : State} (hfr : Frame a b)
    (hd : ∀ x, x ∈ a.decidedViews → x ∈ b.decidedViews)
    (hen : DecideEnabled cfg b.abstract v) : DecideEnabled cfg a.abstract v := by
  obtain ⟨hnd, habove, hc1, c2, p, hc2, hp, hbh⟩ := hen
  refine ⟨fun hc => hnd (hd v hc), fun d hdd => habove d (hd d hdd), ?_,
    c2, p, ?_, ?_, hbh⟩
  · show (a.cert1s.get? v).isSome = true
    rw [← hfr.cert1s]; exact hc1
  · show a.cert2s.get? v = some c2
    rw [← hfr.cert2s]; exact hc2
  · show a.proposals.get? v = some p
    rw [← hfr.proposals]; exact hp

end Impl
end NewProtocol
