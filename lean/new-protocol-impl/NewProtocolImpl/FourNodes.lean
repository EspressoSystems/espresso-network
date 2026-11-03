module

public import NewProtocolSpec.Network

/-!
# What several witnesses share

Block headers, the anchor and a single-epoch configuration, and four nodes `a`,
`b`, `c` and `d` with `d` faulty: one committee of all four, any three a quorum,
which is what one faulty node in four amounts to.
-/

@[expose] public section

namespace NewProtocolImpl
namespace FourNodes

open NewProtocol

/-- The header of the block at height `u`. -/
def hdr (u : Nat) : BlockHeader := ⟨⟨u⟩, ⟨u⟩⟩

/-- Genesis. -/
def anchorB : Block :=
  ⟨⟨⟨0⟩, ⟨0⟩⟩, ViewNumber.genesis, ⟨0⟩, ⟨⟨⟨0⟩, ⟨0⟩, ⟨0⟩⟩, ViewNumber.genesis⟩, none, ⟨7⟩⟩

/-- The certificate over a block, at the block's view. -/
def certOf (b : Block) : Cert1 := ⟨⟨blockHash b, b.epoch, b.blockHeader.blockNumber⟩, b.viewNumber⟩

/-- One epoch for the whole run. -/
def cfg : Config where
  anchorBlock := anchorB
  anchorCert := certOf anchorB
  decideBuffer := 20
  epochHeight := 0

theorem cfg_coherent : ConfigCoherent cfg where
  anchorCertView := rfl
  anchorBlockEpoch := rfl
  anchorCertBlock := rfl
  anchorCertBlockNumber := rfl
  anchorCertEpoch := rfl

/-- The honest nodes. -/
def a : PubKey := ⟨1⟩

def b : PubKey := ⟨2⟩

def c : PubKey := ⟨3⟩

/-- The faulty node. -/
def d : PubKey := ⟨4⟩

/-- Four members, three honest, and any three a quorum. -/
def C : Committee where
  honest _ k := k = a ∨ k = b ∨ k = c
  members _ k := k = a ∨ k = b ∨ k = c ∨ k = d
  Quorum _ q := (q a ∧ q b ∧ q c) ∨ (q a ∧ q b ∧ q d) ∨ (q a ∧ q c ∧ q d) ∨ (q b ∧ q c ∧ q d)
  intersect _ q q' hq hq' := by
    rcases hq with ⟨h1, h2, h3⟩ | ⟨h1, h2, h3⟩ | ⟨h1, h2, h3⟩ | ⟨h1, h2, h3⟩ <;>
    rcases hq' with ⟨h1', h2', h3'⟩ | ⟨h1', h2', h3'⟩ | ⟨h1', h2', h3'⟩ | ⟨h1', h2', h3'⟩ <;>
    first
      | exact ⟨a, by assumption, by assumption, Or.inl rfl⟩
      | exact ⟨b, by assumption, by assumption, Or.inr (Or.inl rfl)⟩
      | exact ⟨c, by assumption, by assumption, Or.inr (Or.inr rfl)⟩
  honestFinite := ⟨[a, b, c], fun _ k hk => by rcases hk with rfl | rfl | rfl <;> simp⟩

/-- The honest members of every epoch's committee are a quorum. -/
theorem members_quorum : ∀ e, C.Quorum e fun k => C.members e k ∧ C.honest e k := fun _ =>
  Or.inl ⟨⟨Or.inl rfl, Or.inl rfl⟩, ⟨Or.inr (Or.inl rfl), Or.inr (Or.inl rfl)⟩,
    ⟨Or.inr (Or.inr (Or.inl rfl)), Or.inr (Or.inr rfl)⟩⟩

theorem honest_quorum : C.Quorum ⟨0⟩ (C.honest ⟨0⟩) :=
  Or.inl ⟨Or.inl rfl, Or.inr (Or.inl rfl), Or.inr (Or.inr rfl)⟩

theorem d_faulty : ¬ C.Honest d := by
  rintro ⟨_, h | h | h⟩ <;> cases h

/-- Every quorum of honest members holds `a`. -/
theorem quorum_a {q : PubKey → Prop} (hq : C.Quorum ⟨0⟩ q) (hh : ∀ k, q k → C.Honest k) : q a := by
  rcases hq with ⟨h1, -, -⟩ | ⟨h1, -, -⟩ | ⟨h1, -, -⟩ | ⟨-, -, h3⟩
  · exact h1
  · exact h1
  · exact h1
  · exact absurd (hh d h3) d_faulty

end FourNodes
end NewProtocolImpl
