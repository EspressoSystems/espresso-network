module

public import StakeTableSpec

/-!
# Computing the block reward

`StakeTable.blockReward` computes the reward with an integer square root
(`StakeTable.roundedRoot`), a half going to the even neighbour as in the node. It
gives the specification's reward, and `none` exactly where there is none
(`StakeTable.blockReward_eq_some_iff`). `StakeTable.rewardAdmitted` decides whether a
number is a block reward the node's arithmetic can give
(`StakeTable.rewardAdmitted_iff`).
-/

@[expose] public section

namespace StakeTable

/-- The block reward, or `none` where the node reports an error. -/
def blockReward (supply stake avgMs : Nat) : Option Nat :=
  let blocks := blocksPerYear avgMs
  if supply = 0 ∨ supply < stake ∨ avgMs = 0 ∨ blocks = 0 then none
  else
    let q := rewardSquare supply stake blocks
    some (roundedRoot q.1 q.2)

/-- The computed reward is the specification's, and there is one exactly where it is defined. -/
theorem blockReward_eq_some_iff {supply stake avgMs r : Nat} :
    blockReward supply stake avgMs = some r ↔ BlockReward supply stake avgMs r := by
  unfold blockReward
  by_cases hc : supply = 0 ∨ supply < stake ∨ avgMs = 0 ∨ blocksPerYear avgMs = 0
  · simp only [hc, ite_true, reduceCtorEq, false_iff]
    intro ⟨⟨h1, h2, h3, h4⟩, _⟩
    omega
  · simp only [hc, ite_false, Option.some.injEq]
    have hdef : RewardDefined supply stake avgMs := by
      unfold RewardDefined; omega
    have hb : 0 < blocksPerYear avgMs := hdef.2.2.2
    have hd : 0 < (rewardSquare supply stake (blocksPerYear avgMs)).2 := by
      unfold rewardSquare
      split <;> exact Nat.mul_pos (Nat.mul_pos (by decide) hb) hb
    have hn :=
      nearestRoot_roundedRoot (num := (rewardSquare supply stake (blocksPerYear avgMs)).1) hd
    constructor
    · intro h; subst h; exact ⟨hdef, hn⟩
    · intro h; exact nearestRoot_unique hn h.2

/--
Whether `n` is a block reward the node's arithmetic can give; `false` where the node
reports an error.
-/
def rewardAdmitted (supply stake avgMs n : Nat) : Bool :=
  let blocks := blocksPerYear avgMs
  if supply = 0 ∨ supply < stake ∨ avgMs = 0 ∨ blocks = 0 then false
  else
    let q := rewardSquare supply stake blocks
    decide ((2 * n - 1) * (2 * n - 1) * (rewardPrecision * rewardPrecision) * q.2
        ≤ 4 * ((rewardPrecision + 1) * (rewardPrecision + 1)) * q.1)
      && decide (4 * ((rewardPrecision - 1) * (rewardPrecision - 1)) * q.1
        ≤ (2 * n + 1) * (2 * n + 1) * (rewardPrecision * rewardPrecision) * q.2)

/-- `rewardAdmitted` decides the specification's admitted block rewards. -/
theorem rewardAdmitted_iff {supply stake avgMs n : Nat} :
    rewardAdmitted supply stake avgMs n = true ↔ AdmittedBlockReward supply stake avgMs n := by
  unfold rewardAdmitted AdmittedBlockReward NearRoot
  by_cases hc : supply = 0 ∨ supply < stake ∨ avgMs = 0 ∨ blocksPerYear avgMs = 0
  · simp only [hc, ite_true, Bool.false_eq_true, false_iff]
    intro ⟨⟨h1, h2, h3, h4⟩, _⟩
    omega
  · have hdef : RewardDefined supply stake avgMs := by
      unfold RewardDefined; omega
    have hb : 0 < blocksPerYear avgMs := hdef.2.2.2
    have hd : 0 < (rewardSquare supply stake (blocksPerYear avgMs)).2 := by
      unfold rewardSquare
      split <;> exact Nat.mul_pos (Nat.mul_pos (by decide) hb) hb
    simp only [hc, ite_false, Bool.and_eq_true, decide_eq_true_eq, hdef, hd, true_and]

end StakeTable
