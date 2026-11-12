module

public import StakeTableSpec.Spec

/-!
# What the block reward is like

At most one integer is nearest a square root, a half going to the even neighbour
(`StakeTable.nearestRoot_unique`), so the inputs fix the block reward
(`StakeTable.blockReward_unique`). One is, computed from an integer square root
(`StakeTable.nearestRoot_roundedRoot`), so where the inputs give a reward there is one
(`StakeTable.blockReward_exists`).

The root of a larger fraction rounds to no less (`StakeTable.nearestRoot_mono`), and
the squared reward does not decrease with stake (`StakeTable.rewardSquare_mono`), so
more stake never lowers the block reward (`StakeTable.blockReward_mono`). The square
is largest with the whole supply staked, which bounds the rewards of a year
(`StakeTable.issuance_bounded`).
-/

@[expose] public section

namespace StakeTable

theorem sq_lt {x y d : Nat} (h : x < y) (hd : 0 < d) : x * x * d < y * y * d :=
  Nat.mul_lt_mul_of_pos_right (Nat.mul_self_lt_mul_self h) hd

theorem sq_le {x y d : Nat} (h : x ≤ y) : x * x * d ≤ y * y * d :=
  Nat.mul_le_mul_right _ (Nat.mul_le_mul h h)

/-- At most one integer is nearest a square root, a half going to the even neighbour. -/
theorem nearestRoot_unique {num den r r' : Nat} (h : NearestRoot num den r)
    (h' : NearestRoot num den r') : r = r' := by
  have key : ∀ {a b : Nat}, NearestRoot num den a → NearestRoot num den b → a < b → False := by
    intro a b ha hb hab
    obtain ⟨hd, -, hau, -, hat⟩ := ha
    obtain ⟨-, hbl, -, hbt, -⟩ := hb
    -- The upper end of `a`'s interval is at most the lower end of `b`'s.
    have hle : 2 * a + 1 ≤ 2 * b - 1 := by omega
    have heq : 2 * a + 1 = 2 * b - 1 := by
      refine Nat.le_antisymm hle (Nat.le_of_not_lt fun hlt => ?_)
      have := sq_lt hlt hd
      omega
    -- Then both ends meet at `4 * num`, a half, and `a` and `b = a + 1` would both be even.
    rw [← heq] at hbl hbt
    have h4 : 4 * num = (2 * a + 1) * (2 * a + 1) * den := Nat.le_antisymm hau hbl
    have := hat h4
    have := hbt h4.symm
    omega
  rcases Nat.lt_trichotomy r r' with hlt | heq | hgt
  · exact (key h h' hlt).elim
  · exact heq
  · exact (key h' h hgt).elim

/--
The root of a fraction rounds to no less than the root of a smaller fraction.

The fractions are compared cross-multiplied.
-/
theorem nearestRoot_mono {num den num' den' r r' : Nat} (hle : num * den' ≤ num' * den)
    (h : NearestRoot num den r) (h' : NearestRoot num' den' r') : r ≤ r' := by
  refine Nat.le_of_not_lt fun hlt => ?_
  obtain ⟨hd, hl, -, ht, -⟩ := h
  obtain ⟨hd', -, hu', -, ht'⟩ := h'
  -- The upper end of `r'`'s interval is at most the lower end of `r`'s.
  have hyx : 2 * r' + 1 ≤ 2 * r - 1 := by omega
  generalize hx : 2 * r - 1 = x at *
  generalize hy : 2 * r' + 1 = y at *
  have h1 : x * x * den * den' ≤ 4 * (num * den') := by
    rw [← Nat.mul_assoc]; exact Nat.mul_le_mul_right den' hl
  have h2 : 4 * (num' * den) ≤ y * y * den' * den := by
    rw [← Nat.mul_assoc]; exact Nat.mul_le_mul_right den hu'
  have h3 : y * y * den' * den ≤ x * x * den * den' := by
    rw [Nat.mul_right_comm (x * x)]; exact Nat.mul_le_mul_right den (sq_le hyx)
  -- Then the ends meet and both fractions are halves, so `r` and `r' = r - 1` would both
  -- be even.
  have he : x = y := by
    refine Nat.le_antisymm (Nat.le_of_not_lt fun hlt' => ?_) hyx
    have := Nat.mul_lt_mul_of_pos_right (sq_lt hlt' hd') hd
    rw [Nat.mul_right_comm (x * x)] at this
    omega
  have := ht (Nat.eq_of_mul_eq_mul_right hd' (by rw [Nat.mul_assoc 4]; omega))
  have := ht' (Nat.eq_of_mul_eq_mul_right hd (by rw [Nat.mul_assoc 4]; omega))
  omega

/-- The inputs fix the block reward. -/
theorem blockReward_unique {supply stake avgMs r r' : Nat} (h : BlockReward supply stake avgMs r)
    (h' : BlockReward supply stake avgMs r') : r = r' :=
  nearestRoot_unique h.2 h'.2

/-- The squared reward does not decrease with stake, the fractions compared cross-multiplied. -/
theorem rewardSquare_mono (supply blocks : Nat) {stake stake' : Nat} (h : stake ≤ stake') :
    (rewardSquare supply stake blocks).1 * (rewardSquare supply stake' blocks).2
      ≤ (rewardSquare supply stake' blocks).1 * (rewardSquare supply stake blocks).2 := by
  unfold rewardSquare
  split <;> split
  · exact Nat.mul_le_mul_right _ (Nat.mul_le_mul_right _ (Nat.mul_le_mul_left _ h))
  · omega
  · -- The smaller stake is below a hundredth of the supply and the larger is not.
    have hs : 100 * stake * stake ≤ supply * stake' := Nat.mul_le_mul (by omega) h
    calc 9 * stake * stake * (20000 * blocks * blocks)
        = 9 * 200 * (blocks * blocks) * (100 * stake * stake) := by
          show 9 * stake * stake * (200 * 100 * blocks * blocks) = _
          ac_rfl
      _ ≤ 9 * 200 * (blocks * blocks) * (supply * stake') := Nat.mul_le_mul_left _ hs
      _ = 9 * stake' * supply * (200 * blocks * blocks) := by ac_rfl
  · exact Nat.mul_le_mul_right _ (Nat.mul_le_mul (Nat.mul_le_mul_left _ h) h)

/-- More stake never lowers the block reward. -/
theorem blockReward_mono {supply stake stake' avgMs r r' : Nat} (hs : stake ≤ stake')
    (h : BlockReward supply stake avgMs r) (h' : BlockReward supply stake' avgMs r') : r ≤ r' :=
  nearestRoot_mono (rewardSquare_mono supply (blocksPerYear avgMs) hs) h.2 h'.2

/--
The integer nearest `√(num / den)`, a half going to the even neighbour, from the
integer square root `t` of `⌊4 num / den⌋`: `⌊(t + 1) / 2⌋`, unless `4 num / den` is
exactly the square of an odd `t`, a half, which goes to the even one of `⌊t / 2⌋` and
`⌊t / 2⌋ + 1`.
-/
def roundedRoot (num den : Nat) : Nat :=
  if 4 * num % den = 0 ∧ Nat.sqrt (4 * num / den) * Nat.sqrt (4 * num / den) = 4 * num / den
      ∧ Nat.sqrt (4 * num / den) % 2 = 1 then
    if Nat.sqrt (4 * num / den) / 2 % 2 = 0 then Nat.sqrt (4 * num / den) / 2
    else Nat.sqrt (4 * num / den) / 2 + 1
  else (Nat.sqrt (4 * num / den) + 1) / 2

theorem nearestRoot_roundedRoot {num den : Nat} (hd : 0 < den) :
    NearestRoot num den (roundedRoot num den) := by
  unfold roundedRoot
  generalize hq : 4 * num / den = q
  generalize ht : Nat.sqrt q = t
  have hsq : t * t ≤ q := ht ▸ Nat.sqrt_le q
  have hlt : q < (t + 1) * (t + 1) := ht ▸ Nat.lt_succ_sqrt q
  have hdm := Nat.div_add_mod (4 * num) den
  have hmod := Nat.mod_lt (4 * num) hd
  rw [hq] at hdm
  have hlo : q * den ≤ 4 * num := by rw [Nat.mul_comm]; omega
  have hhi : 4 * num < (q + 1) * den := by
    have : (q + 1) * den = den * q + den := by rw [Nat.add_mul, Nat.one_mul, Nat.mul_comm]
    omega
  split
  · -- `4 num / den` is exactly `t * t` for an odd `t`: a half, which goes to the even side.
    rename_i htie
    obtain ⟨hm, hsqq, hodd⟩ := htie
    have h4 : 4 * num = t * t * den := by
      rw [hsqq]; rw [Nat.mul_comm q den]; omega
    have hle : ∀ x, x ≤ t → x * x * den ≤ 4 * num := fun x hx => h4 ▸ sq_le hx
    have hge : ∀ x, t ≤ x → 4 * num ≤ x * x * den := fun x hx => h4 ▸ sq_le hx
    split
    · rename_i heven
      exact ⟨hd, hle _ (by omega), hge _ (by omega), fun _ => heven, fun _ => heven⟩
    · rename_i hodd'
      have heven : (t / 2 + 1) % 2 = 0 := by omega
      exact ⟨hd, hle _ (by omega), hge _ (by omega), fun _ => heven, fun _ => heven⟩
  · rename_i hnot
    have hr1 : 2 * ((t + 1) / 2) - 1 ≤ t := by omega
    have hr2 : t + 1 ≤ 2 * ((t + 1) / 2) + 1 := by omega
    have hlower : (2 * ((t + 1) / 2) - 1) * (2 * ((t + 1) / 2) - 1) * den ≤ 4 * num :=
      Nat.le_trans (Nat.le_trans (sq_le hr1) (Nat.mul_le_mul_right _ hsq)) hlo
    have hupper : 4 * num < (2 * ((t + 1) / 2) + 1) * (2 * ((t + 1) / 2) + 1) * den :=
      Nat.lt_of_lt_of_le hhi (Nat.mul_le_mul_right _
        (Nat.le_trans (Nat.succ_le_of_lt hlt) (Nat.mul_le_mul hr2 hr2)))
    refine ⟨hd, hlower, Nat.le_of_lt hupper, fun heq => ?_, fun heq => absurd heq (Nat.ne_of_lt hupper)⟩
    -- An odd `r` with its lower end at `4 * num` would make `4 num / den` the square of
    -- the odd `t`.
    by_cases hr0 : (t + 1) / 2 % 2 = 0
    · exact hr0
    have hte : 2 * ((t + 1) / 2) - 1 = t := by
      refine Nat.le_antisymm hr1 (Nat.le_of_not_lt fun hlt' => ?_)
      have := sq_lt hlt' hd
      have := Nat.le_trans (Nat.mul_le_mul_right den hsq) hlo
      omega
    rw [hte] at heq
    have hm : 4 * num % den = 0 := by rw [← heq]; exact Nat.mul_mod_left _ _
    have hqq : q = t * t := by
      rw [← hq, ← heq, Nat.mul_div_cancel _ hd]
    exact absurd ⟨hm, hqq.symm, by omega⟩ hnot

/-- Where the inputs give a reward, there is one. -/
theorem blockReward_exists {supply stake avgMs : Nat} (h : RewardDefined supply stake avgMs) :
    ∃ r, BlockReward supply stake avgMs r := by
  have hb : 0 < blocksPerYear avgMs := h.2.2.2
  have hd : 0 < (rewardSquare supply stake (blocksPerYear avgMs)).2 := by
    unfold rewardSquare
    split <;> exact Nat.mul_pos (Nat.mul_pos (by decide) hb) hb
  exact ⟨_, h, nearestRoot_roundedRoot hd⟩

/--
The block rewards of a year are at most 2.13% of the supply, and half a unit per
block for rounding.
-/
theorem issuance_bounded {supply stake avgMs r : Nat} (h : BlockReward supply stake avgMs r) :
    10000 * (r * blocksPerYear avgMs) ≤ 213 * supply + 5000 * blocksPerYear avgMs := by
  obtain ⟨⟨-, hs, -, -⟩, -, hl, -⟩ := h
  generalize blocksPerYear avgMs = blocks at *
  -- The lower end of `r`'s interval bounds the square of `(2 * r - 1) * blocks`, by the
  -- square of the supply.
  have hsq : 20000 * ((2 * r - 1) * blocks * ((2 * r - 1) * blocks)) ≤ 36 * (supply * supply) := by
    unfold rewardSquare at hl
    split at hl
    · have he : (2 * r - 1) * (2 * r - 1) * (20000 * blocks * blocks)
          = 20000 * ((2 * r - 1) * blocks * ((2 * r - 1) * blocks)) := by ac_rfl
      have := Nat.mul_le_mul_right supply hs
      rw [he, Nat.mul_assoc 9] at hl
      omega
    · have he : (2 * r - 1) * (2 * r - 1) * (200 * blocks * blocks)
          = 200 * ((2 * r - 1) * blocks * ((2 * r - 1) * blocks)) := by ac_rfl
      have : 100 * stake * (100 * stake) ≤ supply * supply := Nat.mul_le_mul (by omega) (by omega)
      rw [Nat.mul_mul_mul_comm] at this
      rw [he, Nat.mul_assoc 9] at hl
      omega
  have ha : 10000 * ((2 * r - 1) * blocks) ≤ 426 * supply := by
    refine Nat.mul_self_le_mul_self_iff.1 ?_
    rw [Nat.mul_mul_mul_comm, Nat.mul_mul_mul_comm 426]
    omega
  have hr : 2 * (r * blocks) ≤ (2 * r - 1) * blocks + blocks := by
    have := Nat.mul_le_mul_right blocks (show 2 * r ≤ 2 * r - 1 + 1 by omega)
    rwa [Nat.add_mul, Nat.one_mul, Nat.mul_assoc] at this
  omega

end StakeTable
