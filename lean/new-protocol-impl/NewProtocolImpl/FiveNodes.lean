module

public import NewProtocolSpec.Network

/-!
# The nodes and committees of the epoch witnesses

Five nodes: `a`, `b`, `c` and `nw` honest, `d` faulty and silent. The committee of
an odd epoch is `a`, `b`, `c` and `d`, that of an even one `a`, `b`, `nw` and `d`,
and any three members are a quorum. So `c` and `nw` are each outside every other
committee. `a` leads every view. One block to an epoch, unless a witness says
otherwise.
-/

@[expose] public section

namespace NewProtocolImpl
namespace FiveNodes

open NewProtocol

/-! ## Nodes and committees -/

/-- The honest nodes. -/
def a : PubKey := ⟨1⟩

def b : PubKey := ⟨2⟩

def c : PubKey := ⟨3⟩

def nw : PubKey := ⟨5⟩

/-- The faulty node. -/
def d : PubKey := ⟨4⟩

/-- The third honest member of an epoch's committee: `c` in odd epochs, `nw` in even ones. -/
def third (e : EpochNumber) : PubKey := if e.toNat % 2 = 1 then c else nw

/-- `a` leads every view. -/
def leader : EpochNumber → ViewNumber → Option PubKey := fun _ _ => some a

def Honest (k : PubKey) : Prop := k = a ∨ k = b ∨ k = c ∨ k = nw

theorem third_honest (e : EpochNumber) : Honest (third e) := by
  unfold third; split
  · exact Or.inr (Or.inr (Or.inl rfl))
  · exact Or.inr (Or.inr (Or.inr rfl))

/-- Four members to an epoch, three honest, and any three a quorum. -/
def C : Committee where
  honest _ := Honest
  members e k := k = a ∨ k = b ∨ k = third e ∨ k = d
  Quorum e q := (q a ∧ q b ∧ q (third e)) ∨ (q a ∧ q b ∧ q d) ∨ (q a ∧ q (third e) ∧ q d)
    ∨ (q b ∧ q (third e) ∧ q d)
  intersect e q q' hq hq' := by
    rcases hq with ⟨h1, h2, h3⟩ | ⟨h1, h2, h3⟩ | ⟨h1, h2, h3⟩ | ⟨h1, h2, h3⟩ <;>
    rcases hq' with ⟨h1', h2', h3'⟩ | ⟨h1', h2', h3'⟩ | ⟨h1', h2', h3'⟩ | ⟨h1', h2', h3'⟩ <;>
    first
      | exact ⟨a, by assumption, by assumption, Or.inl rfl⟩
      | exact ⟨b, by assumption, by assumption, Or.inr (Or.inl rfl)⟩
      | exact ⟨third e, by assumption, by assumption, third_honest e⟩
  honestFinite := ⟨[a, b, c, nw], fun _ k hk => by rcases hk with rfl | rfl | rfl | rfl <;> simp⟩

/-- The honest members of every epoch's committee are a quorum. -/
theorem members_quorum : ∀ e, C.Quorum e fun k => C.members e k ∧ C.honest e k := fun e =>
  Or.inl ⟨⟨Or.inl rfl, Or.inl rfl⟩, ⟨Or.inr (Or.inl rfl), Or.inr (Or.inl rfl)⟩,
    ⟨Or.inr (Or.inr (Or.inl rfl)), third_honest e⟩⟩

theorem d_faulty : ¬ C.Honest d := by
  rintro ⟨_, h | h | h | h⟩ <;> cases h

/-- Every quorum of honest members holds `a`. -/
theorem quorum_a {e : EpochNumber} {q : PubKey → Prop} (hq : C.Quorum e q)
    (hh : ∀ k, q k → C.Honest k) : q a := by
  rcases hq with ⟨h1, -, -⟩ | ⟨h1, -, -⟩ | ⟨h1, -, -⟩ | ⟨-, -, h3⟩
  · exact h1
  · exact h1
  · exact h1
  · exact absurd (hh d h3) d_faulty

theorem honest_quorum (e : EpochNumber) : C.Quorum e (C.honest e) :=
  Or.inl ⟨Or.inl rfl, Or.inr (Or.inl rfl), third_honest e⟩

/-- `c` is outside the even epochs' committees, and `nw` outside the odd ones'. -/
theorem outside (u : Nat) : ¬ C.members ⟨2 * u + 2⟩ c ∧ ¬ C.members ⟨2 * u + 1⟩ nw := by
  have h2 : third ⟨2 * u + 2⟩ = nw := by
    unfold third; split
    · rename_i h; exact absurd h (by show ¬ (2 * u + 2) % 2 = 1; omega)
    · rfl
  have h1 : third ⟨2 * u + 1⟩ = c := by
    unfold third; split
    · rfl
    · rename_i h; exact absurd (by show (2 * u + 1) % 2 = 1; omega) h
  refine ⟨fun h => ?_, fun h => ?_⟩
  · simp only [C, h2] at h
    rcases h with h | h | h | h <;> cases h
  · simp only [C, h1] at h
    rcases h with h | h | h | h <;> cases h

/-- Genesis: block zero, in epoch one. -/
def anchorB : Block :=
  ⟨⟨⟨0⟩, ⟨0⟩⟩, ViewNumber.genesis, ⟨1⟩, ⟨⟨⟨0⟩, ⟨0⟩, ⟨0⟩⟩, ViewNumber.genesis⟩, none, ⟨7⟩⟩

/-- The certificate over a block, at the block's view. -/
def certOf (x : Block) : Cert1 := ⟨⟨blockHash x, x.epoch, x.blockHeader.blockNumber⟩, x.viewNumber⟩

/-- One block to an epoch. -/
def cfg : Config where
  anchorBlock := anchorB
  anchorCert := certOf anchorB
  decideBuffer := 20
  epochHeight := 1

theorem cfg_coherent : ConfigCoherent cfg where
  anchorCertView := rfl
  anchorBlockEpoch := rfl
  anchorCertBlock := rfl
  anchorCertBlockNumber := rfl
  anchorCertEpoch := rfl

/-- With one block to an epoch, block `n` is in epoch `n`. -/
theorem epochOf_one_height {n : Nat} (hn : n ≠ 0) : epochOf ⟨n⟩ 1 = ⟨n⟩ := by
  simp [epochOf_eq, hn, Nat.mod_one]

theorem last_block {n : Nat} (hn : n ≠ 0) : IsLastBlock ⟨n⟩ cfg.epochHeight :=
  isLastBlock_iff.mpr ⟨hn, by decide, Nat.mod_one n⟩

/--
How many steps after the committee a node outside block `u + 1`'s committee can lock
on the block: none for a member, which rebuilds the payload, and two for a
non-member, which locks on the epoch change.
-/
def lag (k : PubKey) (u : Nat) : Nat := if k = third ⟨u + 2⟩ then 2 else 0

theorem third_cases (e : EpochNumber) : third e = c ∨ third e = nw := by
  unfold third; split
  · exact Or.inl rfl
  · exact Or.inr rfl

theorem lag_cases (k : PubKey) (u : Nat) : lag k u = 0 ∨ lag k u = 2 := by
  unfold lag; split
  · exact Or.inr rfl
  · exact Or.inl rfl

theorem third_succ (u : Nat) : third ⟨u + 1⟩ ≠ third ⟨u + 2⟩ := by
  unfold third
  by_cases h : (u + 1) % 2 = 1
  · rw [ite_eq_left h, ite_eq_right (show ¬ (u + 2) % 2 = 1 by omega)]; decide
  · rw [ite_eq_right h, ite_eq_left (show (u + 2) % 2 = 1 by omega)]; decide

/-- A member of block `u + 1`'s committee has the payload with the committee. -/
theorem lag_member {k : PubKey} {u : Nat} (hm : C.members ⟨u + 1⟩ k) : lag k u = 0 := by
  unfold lag
  rw [ite_eq_right]
  intro he
  rcases hm with rfl | rfl | rfl | rfl
  · rcases third_cases ⟨u + 2⟩ with h | h <;> rw [h] at he <;> cases he
  · rcases third_cases ⟨u + 2⟩ with h | h <;> rw [h] at he <;> cases he
  · exact third_succ u he
  · rcases third_cases ⟨u + 2⟩ with h | h <;> rw [h] at he <;> cases he

theorem lag_a (u : Nat) : lag a u = 0 := lag_member (Or.inl rfl)

/-- A node outside block `u + 1`'s committee. -/
theorem lag_outside {k : PubKey} {u : Nat} (h : lag k u = 2) : ¬ C.members ⟨u + 1⟩ k :=
  fun hm => by rw [lag_member hm] at h; cases h

theorem leader_eq {e : EpochNumber} {v : ViewNumber} {k : PubKey} (h : leader e v = some k) : k = a :=
  (Option.some.inj h).symm

end FiveNodes
end NewProtocolImpl
