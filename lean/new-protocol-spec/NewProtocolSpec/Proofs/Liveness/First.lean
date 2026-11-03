module

public import NewProtocolSpec.Proofs.Liveness.Basic
public import NewProtocolSpec.Proofs.Votes

/-!
# When a view is first reached, and when it may be given up

The first time an honest node reaches a view (`Liveness.firstReach`), and the
fact the liveness argument starts from: no honest node times that view or a later
one out, and none holds a timeout certificate for one, until `τ` after it
(`Liveness.not_past`). Also the small facts about held certificates every later
section reads. Nothing here depends on epochs.
-/

@[expose] public section

namespace NewProtocol
namespace Liveness

open History Lists

variable {cfg : Config} {leader : EpochNumber → ViewNumber → Option PubKey} {C : Committee}
variable (N : TimedNetwork cfg leader C)

/-! ## The first node in a view, and when anyone may give it up -/

section First

/-- After `n` steps node `k` has grounds for view `w` or a later one. -/
def Reached (k : PubKey) (hk : C.Honest k) (n : Nat) (w : ViewNumber) : Prop :=
  ∃ v, ((N.trace k hk).history n).ViewGround cfg v ∧ w ≤ v

/-- Some honest node reached `w` with steps all by `T`. -/
def ReachedBy (T : Nat) (w : ViewNumber) : Prop :=
  ∃ k hk n, (∀ i, i < n → N.time k hk i ≤ T) ∧ Reached N k hk n w

/-- The first time an honest node reaches `w`. -/
noncomputable def firstReach {w : ViewNumber} (hex : ∃ T, ReachedBy N T w) : Nat :=
  least (fun T => ReachedBy N T w) hex

variable {N}

theorem reached_mono {k : PubKey} {hk : C.Honest k} {m n : Nat} {w : ViewNumber} (hle : m ≤ n)
    (hr : Reached N k hk m w) : Reached N k hk n w := by
  obtain ⟨v, hv, hwv⟩ := hr
  exact ⟨v, viewGround_grows (received_grows _ hle) hv, hwv⟩

/-- The empty history has grounds only for the view after the anchor's. -/
theorem ground_nil (hcfg : ConfigCoherent cfg) {r : Trace} {v : ViewNumber}
    (hv : (r.history 0).ViewGround cfg v) : v.toNat ≤ cfg.anchorView.toNat + 1 := by
  have hnr : ∀ i, ¬ (r.history 0).Received i := fun i hr => by
    obtain ⟨j, hj, -⟩ := (Trace.received_history r).mp hr; exact absurd hj (Nat.not_lt_zero _)
  rcases hv with ⟨c, hc, rfl⟩ | ⟨tc, htc, _⟩ | ⟨c1, c2, p, ⟨hrec, _⟩, _⟩
  · obtain rfl := hasCert1_nil hc
    show cfg.anchorCert.view.toNat + 1 ≤ cfg.anchorView.toNat + 1
    rw [hcfg.anchorCertView]; exact Nat.le_refl _
  · exact absurd htc (hnr _)
  · exact absurd hrec (hnr _)

theorem not_reached_nil (hcfg : ConfigCoherent cfg) {k : PubKey} {hk : C.Honest k}
    {w : ViewNumber} (hw : cfg.anchorView.toNat + 2 ≤ w.toNat) : ¬ Reached N k hk 0 w := by
  rintro ⟨v, hv, hwv⟩
  have := ground_nil hcfg hv
  have : w.toNat ≤ v.toNat := hwv
  omega

theorem firstReach_le {w : ViewNumber} (hex : ∃ T, ReachedBy N T w) {k : PubKey} {hk : C.Honest k}
    {n : Nat} (hr : Reached N k hk (n + 1) w) : firstReach N hex ≤ N.time k hk n :=
  least_le hex ⟨k, hk, n + 1, fun _ hi => time_mono N k hk (Nat.le_of_lt_succ hi), hr⟩

variable (hcfg : ConfigCoherent cfg) {GST Δ τ : Nat} (hs : Synchrony N GST Δ τ)
include hcfg hs

/-- `Liveness.timeout_late`, with the induction counter in the statement. -/
private theorem timeout_late_aux {w : ViewNumber} (hw : cfg.anchorView.toNat + 2 ≤ w.toNat) (hex : ∃ T, ReachedBy N T w) :
    ∀ T k (hk : C.Honest k) m vote, C.honest vote.data.epoch k → N.time k hk m = T →
      Output.send (.timeoutVote vote) ∈ (N.trace k hk m).output → w ≤ vote.view →
      firstReach N hex + τ ≤ T := by
  intro T
  induction T using Nat.strongRecOn with
  | ind T ih =>
    intro k hk m vote he hT hm hwv
    obtain ⟨-, -, -, hcase⟩ := (N.protocol k hk (m + 1)).timeoutJustified m (N.trace k hk m) vote
      (Trace.history_getElem? _ (Nat.lt_succ_self m)) hm he
    rw [Trace.history_upTo _ (Nat.le_succ m)] at hcase
    rcases hcase with ⟨hin, hview⟩ | ⟨hin, -⟩
    · have h0 : ¬ ((N.trace k hk).history 0).InView cfg vote.view := fun h0 => by
        have := ground_nil hcfg h0.1
        have : w.toNat ≤ vote.view.toNat := hwv
        omega
      obtain ⟨n, hnm, hin1, hnot⟩ := entered _ hview h0
      have hlate := hs.timerNotEarly k hk n m vote.view hin1 (Or.inr hnot) (Nat.le_of_lt hnm) hin
      have hfirst := firstReach_le hex (n := n) ⟨vote.view, hin1.1, hwv⟩
      omega
    · obtain ⟨k', e, hk'e, m', vote', hve, hm', hv', htm⟩ := N.oneHonestCausal k hk m vote.view hin
      have := ih (N.time k' (.of hk'e) m') (hT ▸ htm) k' (.of hk'e) m' vote' (by rw [hve]; exact hk'e)
        rfl hm' (hv' ▸ hwv)
      omega

/--
No timeout vote for `w` or later, by a node honest in the vote's epoch, comes
before `τ` after the first node reached `w`.
-/
theorem timeout_late {w : ViewNumber} (hw : cfg.anchorView.toNat + 2 ≤ w.toNat) (hex : ∃ T, ReachedBy N T w)
    {k : PubKey} {hk : C.Honest k} {m : Nat} {vote : TimeoutVote} (he : C.honest vote.data.epoch k)
    (hm : Output.send (.timeoutVote vote) ∈ (N.trace k hk m).output) (hwv : w ≤ vote.view) :
    firstReach N hex + τ ≤ N.time k hk m :=
  timeout_late_aux hcfg hs hw hex _ k hk m vote he rfl hm hwv

/-- No honest node holds a timeout certificate for `w` or later until after `τ` from the first node in `w`. -/
theorem timeoutCert_late {w : ViewNumber} (hw : cfg.anchorView.toNat + 2 ≤ w.toNat) (hex : ∃ T, ReachedBy N T w)
    {k : PubKey} {hk : C.Honest k} {m : Nat} {tc : TimeoutCert}
    (hin : (N.trace k hk m).input = .timeoutCertificate tc) (hwv : w ≤ tc.view) :
    firstReach N hex + τ < N.time k hk m := by
  obtain ⟨q, hq, hvotes⟩ := N.timeoutCertCausal k hk m tc hin
  obtain ⟨k', hqk, -, hk'⟩ := C.intersect _ q q hq hq
  obtain ⟨m', vote, ⟨-, hvv, hve, -⟩, hm', htm⟩ := hvotes k' hqk hk'
  have := timeout_late hcfg hs hw hex (hk := .of hk') (by rw [hve]; exact hk') hm' (hvv ▸ hwv)
  omega

/--
Before then, no honest node has given up `w` or any later view, counting what it
signed for the epochs it is honest in.
-/
theorem not_past {w : ViewNumber} (hw : cfg.anchorView.toNat + 2 ≤ w.toNat) (hex : ∃ T, ReachedBy N T w)
    {k : PubKey} {hk : C.Honest k} {n : Nat}
    (hn : ∀ i, i < n → N.time k hk i < firstReach N hex + τ) {v : ViewNumber} (hwv : w ≤ v) :
    ¬ (((N.trace k hk).history n).restrict (C.honest · k)).PastView v := by
  rintro (⟨vote, hs', hle⟩ | ⟨tc, htc, hle⟩)
  · obtain ⟨hs', he⟩ := restrict_sent.mp hs'
    obtain ⟨i, hi, hm⟩ := (Trace.sent_history _).mp hs'
    have := timeout_late hcfg hs hw hex he hm (Nat.le_trans hwv hle)
    have := hn i hi
    omega
  · obtain ⟨i, hi, hin⟩ := (Trace.received_history _).mp ((sameInputs_restrict.received _).mp htc)
    have := timeoutCert_late hcfg hs hw hex hin (Nat.le_trans hwv hle)
    have := hn i hi
    omega

end First


/-! ## Small facts -/

theorem anchor_lt {w : ViewNumber} (hw : cfg.anchorView.toNat + 2 ≤ w.toNat) {v : ViewNumber}
    (hv : w ≤ v) : cfg.anchorView < v := by
  show cfg.anchorView.toNat < v.toNat
  have : w.toNat ≤ v.toNat := hv
  omega

theorem by_later {N : TimedNetwork cfg leader C} {k : PubKey} {hk : C.Honest k} {T T' : Nat}
    {P : History → Prop} (hle : T ≤ T') (hby : N.By k hk T P) : N.By k hk T' P := by
  obtain ⟨n, hn, hp⟩ := hby
  exact ⟨n, fun i hi => Nat.le_trans (hn i hi) hle, hp⟩

section Held

variable {N}
variable (hcfg : ConfigCoherent cfg)
include hcfg

/-- A history that can lock on a certificate after genesis is not empty. -/
theorem lockable_pos {r : Trace} {n : Nat} {c : Cert1} (hgen : cfg.anchorView < c.view)
    (hl : (r.history n).Lockable cfg c) : n ≠ 0 := by
  rintro rfl
  rw [lockable_nil hl, hcfg.anchorCertView] at hgen
  exact absurd hgen (Nat.lt_irrefl _)

/-- A `Cert1` an honest node holds, at a view after genesis, is backed. -/
theorem held_backed {k : PubKey} {hk : C.Honest k} {n : Nat} {c : Cert1}
    (hc : ((N.trace k hk).history n).HasCert1 cfg c) (hv : cfg.anchorView < c.view) :
    Cert1Backed N.trace c := by
  rcases cert1_held_backed N.toNetwork hc with rfl | hb
  · rw [hcfg.anchorCertView] at hv; exact absurd hv (Nat.lt_irrefl _)
  · exact hb

/-- A vote2 of an epoch its node is honest in carries the data of a backed `Cert1` at its view. -/
theorem vote2_backed {k : PubKey} {hk : C.Honest k} {v : Vote2}
    (hs : SentBy (N.trace k hk) (.vote2 v)) (he : C.honest v.data.epoch k) :
    ∃ c, Cert1Backed N.trace c ∧ c.view = v.view ∧ v.data = c.data.toVote2 := by
  obtain ⟨-, hgen, c, hview, hdata, n, hc⟩ := vote2_signed N.toNetwork hs he
  exact ⟨c, held_backed hcfg hc (hview ▸ hgen), hview.symm, hdata⟩

/-- A `Cert2` an honest node holds carries the data of a backed `Cert1` at its view. -/
theorem cert2_backed_data {k : PubKey} {hk : C.Honest k} {n : Nat} {c2 : Cert2}
    (hc : ((N.trace k hk).history n).HasCert2 c2) :
    ∃ c, Cert1Backed N.trace c ∧ c.view = c2.view ∧ c2.data = c.data.toVote2 := by
  obtain ⟨q, hq, hvotes⟩ := cert2_held_backed N.toNetwork hc
  obtain ⟨k', hqk, -, hk'⟩ := C.intersect _ q q hq hq
  obtain ⟨c, hb, hv, hd⟩ := vote2_backed hcfg (hvotes k' hqk hk') hk'
  exact ⟨c, hb, hv, hd⟩

end Held

end Liveness
end NewProtocol
