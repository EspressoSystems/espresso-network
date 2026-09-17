module

public import NewProtocolImpl.Conformance.Cases

/-!
# The decide walk

What `Impl.State.chainFrom` returns: held blocks, none of them settled before
the step and none below the floor, linked back to front, headed by the block that
was asked for — and stopping only where `StepSpec.decideJustified` allows a chain
to stop.

These are the four things that obligation asks of a decide event beyond its head:
`ChainLinked`, that every block delivered is one the node holds and had not
decided, and that the oldest block's parent is settled, floored, or not in hand.
All of them are inductions on the walk's fuel, and the keying half of the
representation invariant is what turns "the entry filed under `v`" into "the
block whose view is `v`".

Exhausted fuel is not a fifth stopping place: a held proposal above genesis is
well-formed, so each link strictly descends, and a walk given fuel equal to its
first view meets the floor guard at genesis before the fuel runs out.
-/

@[expose] public section

set_option linter.unusedSectionVars false

open Std (TreeMap TreeSet)

namespace NewProtocol
namespace Impl

variable {s : State} {settled : TreeSet ViewNumber} {floor : ViewNumber}

/-- Every block the walk returns is held, unsettled, and above the floor. -/
theorem chainFrom_mem (hwf : WF s) :
    ∀ (fuel : Nat) (h : BlockHash) (v : ViewNumber) (b : Block),
      b ∈ s.chainFrom settled floor fuel h v →
        s.proposals.get? b.viewNumber = some b ∧ b.viewNumber ∉ settled ∧ floor < b.viewNumber := by
  intro fuel
  induction fuel with
  | zero => intro h v b hb; exact absurd hb (by simp [State.chainFrom])
  | succ fuel ih =>
    intro h v b hb
    rw [State.chainFrom] at hb
    split at hb
    · exact absurd hb (by simp)
    · rename_i hguard
      have hfl : floor < v := Nat.lt_of_not_le fun hc => hguard (Or.inl hc)
      have hst : v ∉ settled := fun hc => hguard (Or.inr (contains_iff_mem.mpr hc))
      split at hb
      · rename_i q hq
        split at hb
        · rename_i hbh
          rcases List.mem_cons.mp hb with rfl | hb
          · have hv : b.viewNumber = v := hwf.proposals _ _ hq
            exact ⟨by rw [hv]; exact hq, by rw [hv]; exact hst, by rw [hv]; exact hfl⟩
          · exact ih _ _ b hb
        · exact absurd hb (by simp)
      · exact absurd hb (by simp)

/-- The walk starts at the block it was asked for. -/
theorem chainFrom_head (hwf : WF s) :
    ∀ (fuel : Nat) (h : BlockHash) (v : ViewNumber) (b : Block),
      (s.chainFrom settled floor fuel h v).head? = some b →
        b.viewNumber = v ∧ blockHash b = h := by
  intro fuel
  induction fuel with
  | zero => intro h v b hb; exact absurd hb (by simp [State.chainFrom])
  | succ fuel ih =>
    intro h v b hb
    rw [State.chainFrom] at hb
    split at hb
    · exact absurd hb (by simp)
    · split at hb
      · rename_i q hq
        split at hb
        · rename_i hbh
          obtain rfl : b = q := by simpa using hb.symm
          exact ⟨hwf.proposals _ _ hq, hbh⟩
        · exact absurd hb (by simp)
      · exact absurd hb (by simp)

/-- The walk is a `parentCert`-linked chain. -/
theorem chainFrom_linked (hwf : WF s) :
    ∀ (fuel : Nat) (h : BlockHash) (v : ViewNumber),
      ChainLinked (s.chainFrom settled floor fuel h v) := by
  intro fuel
  induction fuel with
  | zero => intro h v; rw [State.chainFrom]; exact trivial
  | succ fuel ih =>
    intro h v
    rw [State.chainFrom]
    split
    · exact trivial
    · split
      · rename_i q hq
        split
        · -- `q`, then the walk from its parent
          cases hrest : s.chainFrom settled floor fuel q.parentCert.data.blockHash
              q.parentCert.view with
          | nil => exact trivial
          | cons b' rest =>
            obtain ⟨hv, hbh⟩ := chainFrom_head hwf fuel q.parentCert.data.blockHash
              q.parentCert.view b' (by rw [hrest]; rfl)
            refine ⟨hv.symm, hbh.symm, ?_⟩
            have := ih q.parentCert.data.blockHash q.parentCert.view
            rw [hrest] at this
            exact this
        · exact trivial
      · exact trivial

/-- An empty walk was asked for a block that is settled, floored, or not in hand. -/
theorem chainFrom_nil {fuel : Nat} {h : BlockHash} {v : ViewNumber} (hle : v.toNat ≤ fuel)
    (hnil : s.chainFrom settled floor fuel h v = []) :
    v ∈ settled ∨ v ≤ floor ∨ ¬ ∃ q, s.proposals.get? v = some q ∧ blockHash q = h := by
  cases fuel with
  | zero => exact Or.inr (Or.inl (Nat.le_trans hle (Nat.zero_le _)))
  | succ fuel =>
    rw [State.chainFrom] at hnil
    split at hnil
    · rename_i hguard
      rcases hguard with hfl | hc
      · exact Or.inr (Or.inl hfl)
      · exact Or.inl (contains_iff_mem.mp hc)
    · split at hnil
      · rename_i q hq
        split at hnil
        · exact absurd hnil (by simp)
        · rename_i hbh
          refine Or.inr (Or.inr fun ⟨q', hq', hb'⟩ => hbh ?_)
          rw [hq, Option.some_inj] at hq'
          exact hq' ▸ hb'
      · rename_i hq
        refine Or.inr (Or.inr fun ⟨q', hq', _⟩ => absurd (hq ▸ hq') (by simp))

/-- The oldest block of a walk has a parent that is settled, floored, or not in hand. -/
theorem chainFrom_last (hwf : WF s) :
    ∀ (fuel : Nat) (h : BlockHash) (v : ViewNumber), v.toNat ≤ fuel →
      ∀ last, (s.chainFrom settled floor fuel h v).getLast? = some last →
        last.parentCert.view ∈ settled ∨ last.parentCert.view ≤ floor
          ∨ ¬ ∃ q, s.proposals.get? last.parentCert.view = some q
              ∧ blockHash q = last.parentCert.data.blockHash := by
  intro fuel
  induction fuel with
  | zero => intro h v hle last hlast; exact absurd hlast (by simp [State.chainFrom])
  | succ fuel ih =>
    intro h v hle last hlast
    rw [State.chainFrom] at hlast
    split at hlast
    · exact absurd hlast (by simp)
    · rename_i hguard
      have hfl : floor < v := Nat.lt_of_not_le fun hc => hguard (Or.inl hc)
      split at hlast
      · rename_i q hq
        split at hlast
        · -- The walk descends, so the fuel is enough for what is left of it.
          have hne : v ≠ ViewNumber.genesis := fun he => absurd (he ▸ hfl) (Nat.not_lt_zero _)
          have hwfq := (wellFormed_iff.mp (hwf.proposalsWellFormed v q hq hne)).1
          have hle' : q.parentCert.view.toNat ≤ fuel :=
            Nat.lt_succ_iff.mp (Nat.lt_of_lt_of_le (hwf.proposals v q hq ▸ hwfq) hle)
          cases hrest : s.chainFrom settled floor fuel q.parentCert.data.blockHash
              q.parentCert.view with
          | nil =>
            rw [hrest] at hlast
            obtain rfl : q = last := by simpa using hlast
            exact chainFrom_nil hle' hrest
          | cons b' rest' =>
            rw [hrest, List.getLast?_cons_cons, ← hrest] at hlast
            exact ih _ _ hle' last hlast
        · exact absurd hlast (by simp)
      · exact absurd hlast (by simp)

/-- Every block of a decided chain is marked decided by the fold that records it. -/
theorem mem_decideFold_of_mem {l : List Block} {d : TreeSet ViewNumber} {b : Block}
    (hb : b ∈ l) : b.viewNumber ∈ l.foldl (fun d b => d.insert b.viewNumber) d := by
  induction l generalizing d with
  | nil => exact absurd hb (by simp)
  | cons a l ih =>
    rcases List.mem_cons.mp hb with rfl | hb
    · show b.viewNumber ∈ l.foldl (fun d b => d.insert b.viewNumber) (d.insert b.viewNumber)
      exact mem_decideFold mem_insert_self
    · exact ih hb

/-- A view marked decided by the fold is one the chain delivered, or one already there. -/
theorem mem_decideFold_cases {l : List Block} {d : TreeSet ViewNumber} {v : ViewNumber}
    (h : v ∈ l.foldl (fun d b => d.insert b.viewNumber) d) :
    v ∈ d ∨ ∃ b ∈ l, b.viewNumber = v := by
  induction l generalizing d with
  | nil => exact Or.inl h
  | cons a l ih =>
    rcases ih (show v ∈ l.foldl (fun d b => d.insert b.viewNumber) (d.insert a.viewNumber) from h)
      with hd | ⟨b, hb, hbv⟩
    · rcases mem_insert.mp hd with rfl | hd
      · exact Or.inr ⟨a, List.mem_cons_self .., rfl⟩
      · exact Or.inl hd
    · exact Or.inr ⟨b, List.mem_cons_of_mem _ hb, hbv⟩

/--
The chain a decide delivers, in one place: linked, and every block held,
unsettled and above the floor.
-/
theorem decideChain_spec (hwf : WF s) {p : Proposal}
    (hp : s.proposals.get? p.viewNumber = some p) (hst : p.viewNumber ∉ settled)
    (hfl : floor < p.viewNumber) :
    ChainLinked (s.decideChain settled floor p)
      ∧ ∀ b ∈ s.decideChain settled floor p,
          s.proposals.get? b.viewNumber = some b ∧ b.viewNumber ∉ settled
            ∧ floor < b.viewNumber := by
  refine ⟨?_, ?_⟩
  · rw [State.decideChain]
    cases hrest : s.chainFrom settled floor p.parentCert.view.toNat p.parentCert.data.blockHash
        p.parentCert.view with
    | nil => exact trivial
    | cons b' rest =>
      obtain ⟨hv, hbh⟩ := chainFrom_head hwf p.parentCert.view.toNat p.parentCert.data.blockHash
        p.parentCert.view b' (by rw [hrest]; rfl)
      refine ⟨hv.symm, hbh.symm, ?_⟩
      have := chainFrom_linked (settled := settled) (floor := floor) hwf
        p.parentCert.view.toNat p.parentCert.data.blockHash p.parentCert.view
      rw [hrest] at this
      exact this
  · intro b hb
    rw [State.decideChain] at hb
    rcases List.mem_cons.mp hb with rfl | hb
    · exact ⟨hp, hst, hfl⟩
    · exact chainFrom_mem hwf _ _ _ b hb

/--
The chain a decide delivers stops where the specification allows.

The chain is the block itself followed by the walk, so its oldest block is the
walk's — or the block itself, when the walk was empty from the start.
-/
theorem decideChain_last (hwf : WF s) {p : Proposal} {last : Block}
    (hlast : (s.decideChain settled floor p).getLast? = some last) :
    last.parentCert.view ∈ settled ∨ last.parentCert.view ≤ floor
      ∨ ¬ ∃ q, s.proposals.get? last.parentCert.view = some q
          ∧ blockHash q = last.parentCert.data.blockHash := by
  rw [State.decideChain] at hlast
  cases hrest : s.chainFrom settled floor p.parentCert.view.toNat p.parentCert.data.blockHash
      p.parentCert.view with
  | nil =>
    rw [hrest] at hlast
    obtain rfl : (p : Block) = last := by simpa using hlast
    exact chainFrom_nil (Nat.le_refl _) hrest
  | cons b' rest' =>
    rw [hrest, List.getLast?_cons_cons, ← hrest] at hlast
    exact chainFrom_last hwf _ _ _ (Nat.le_refl _) last hlast

end Impl
end NewProtocol
