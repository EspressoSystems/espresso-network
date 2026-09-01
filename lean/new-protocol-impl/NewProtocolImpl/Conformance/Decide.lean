module

public import NewProtocolImpl.Conformance.Propose
public import NewProtocolImpl.Conformance.Chain

/-!
# Deciding

Everything `StepSpec.decideJustified` asks about an emitted decide event.

Most of it is judged against the state *before* the step: every block delivered
must have been held, undecided and above the floor then. That is why the decide
round takes the pre-step decided set as a parameter and derives the floor from it
(`Impl.floorOf`) — a decide earlier in the same round moves both, and reading
either from the state the round is standing on would let a later decide claim
ground the round itself had just laid.

The chain's shape comes from `NewProtocolImpl.Conformance.Chain`; this file is
the transfer, plus the head facts, which are where the keying half of the
invariant is used: a `Cert2` filed under `v` names view `v`, and so does the
proposal.
-/

@[expose] public section

set_option linter.unusedSectionVars false

open Std (TreeMap TreeSet)

namespace NewProtocol
namespace Impl

variable {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
variable {settled : TreeSet ViewNumber} {t : State}

/-! ## Reading one watermark from another

The floor of the state the pass began at bounds every state it grows into, in
both directions: a view at or below it is below the floor of anything later, and
a view above it is above the floor of anything earlier.
-/

/-- A view at or below one state's floor is below the floor of everything it grows into. -/
theorem not_aboveDecideFloor_of_le {w u : State} (hwf : WF cfg w)
    (hsub : ∀ d, d ∈ w.decidedViews → d ∈ u.decidedViews) {x : ViewNumber}
    (h : x ≤ w.floor cfg) : ¬ u.abstract.aboveDecideFloor cfg x := fun hab =>
  absurd (hab w.lastDecided (hsub _ (lastDecided_mem hwf.decided))) (Nat.not_lt.mpr h)

/-- A view above one state's floor is above the floor of everything that grew into it. -/
theorem aboveDecideFloor_of_floor_lt {w u : State}
    (hsub : ∀ d, d ∈ u.decidedViews → d ∈ w.decidedViews) {x : ViewNumber}
    (h : w.floor cfg < x) : u.abstract.aboveDecideFloor cfg x :=
  fun d hd => Nat.lt_of_le_of_lt (sub_le_sub _ (le_lastDecided (hsub d hd))) h

/--
A decide event in a pass's output: the chain it delivers, and every fact the
specification asks of it.

Everything is read at `t`, the state the pass began at, which is also the
watermark every attempt of the decide round was judged against — so one round
can neither claim the ground it has just laid nor be judged against ground a
later attempt lays.
-/
theorem pass_decide (hwf : WF cfg t) {chain : List Block} {c1 : Cert1} {c2 : Cert2}
    (h : Output.decided chain c1 c2 ∈ (seq (rounds cfg leader node t) t).2) :
    ∃ head rest, chain = head :: rest
      ∧ c2.view = head.viewNumber
      ∧ c2.data.blockHash = blockHash head
      ∧ t.cert2s.get? head.viewNumber = some c2
      ∧ t.cert1s.get? head.viewNumber = some c1
      ∧ ChainLinked chain
      ∧ (∀ last, chain.getLast? = some last →
          last.parentCert.view ∈ t.decidedViews ∨ last.parentCert.view ≤ t.floor cfg
            ∨ ¬ ∃ q, t.proposals.get? last.parentCert.view = some q
                ∧ blockHash q = last.parentCert.data.blockHash)
      ∧ ∀ b ∈ chain, t.floor cfg < b.viewNumber ∧ b.viewNumber ∉ t.decidedViews
          ∧ b.viewNumber ∈ (st5 cfg leader node t).decidedViews
          ∧ t.proposals.get? b.viewNumber = some b := by
  rw [pass_out] at h
  simp only [List.mem_append] at h
  -- Only the decide round emits a decide.
  rcases h with ((((h | h) | h) | h) | h)
  · obtain ⟨f, hf, u, hwfu, hfrt, hlet, -, ho, -, hle', -⟩ :=
      mem_seq (fun _ => ()) (fs := dSeg cfg t)
        (fun g hg => by
          obtain ⟨v, -, rfl⟩ := List.mem_map.mp
            (show g ∈ List.map (tryDecide cfg t.decidedViews) t.cert2s.keys from hg)
          exact tryDecide_grows)
        (fun _ _ _ => rfl) hwf h
    obtain ⟨v, -, rfl⟩ :=
      List.mem_map.mp (show f ∈ List.map (tryDecide cfg t.decidedViews) t.cert2s.keys from hf)
    rcases tryDecide_cases (r := tryDecide cfg t.decidedViews v u) rfl with heq |
      ⟨p, c1', c2', chain', hp, h1, h2, hbh, hdec, hfl, hchain, heq⟩
    · rw [heq] at ho; exact absurd ho (by simp)
    · rw [heq] at ho
      obtain ⟨rfl, rfl, rfl⟩ : chain = chain' ∧ c1 = c1' ∧ c2 = c2' := by
        simpa [and_assoc] using ho
      -- the head is the proposal at `v`, and the certificate names that view
      have hpv : p.viewNumber = v := hwfu.proposals _ _ hp
      have hc2v : c2.view = v := hwfu.cert2s _ _ h2
      -- `v` was undecided at `t`, since it is undecided even where the attempt ran
      have hst : v ∉ t.decidedViews := fun hc => hdec (hlet.decided _ hc)
      obtain ⟨hlinked, hblocks⟩ :=
        decideChain_spec (settled := t.decidedViews) (floor := floorOf cfg t.decidedViews) hwfu
          (by rw [hpv]; exact hp) (by rw [hpv]; exact hst) (by rw [hpv]; exact hfl)
      rw [← hchain] at hlinked hblocks
      -- the chain starts at the proposal decided
      have hcons : chain = p :: u.chainFrom t.decidedViews (floorOf cfg t.decidedViews)
          p.parentCert.view.toNat p.parentCert.data.blockHash p.parentCert.view := by
        rw [hchain]; rfl
      refine ⟨p, _, hcons, by rw [hc2v, hpv], hbh, ?_, ?_, hlinked, ?_, ?_⟩
      · rw [hpv, ← hfrt.cert2s]; exact h2
      · rw [hpv, ← hfrt.cert1s]; exact h1
      · -- the walk stopped somewhere the specification allows, read at `u`, whose
        -- holdings are `t`'s: the pass frames the proposals.
        intro last hl
        rw [hchain] at hl
        refine (decideChain_last hwfu hl).imp id
          (Or.imp id fun hnh ⟨q, hq, hbq⟩ => hnh ⟨q, ?_, hbq⟩)
        rw [hfrt.proposals]; exact hq
      · intro b hb
        obtain ⟨hheld, hnst, hafl⟩ := hblocks b hb
        refine ⟨hafl, hnst, ?_, ?_⟩
        · refine (st1_to_st5 hwf).decided _ (hle'.decided _ ?_)
          rw [heq]
          exact mem_decideFold_of_mem hb
        · rw [← hfrt.proposals]; exact hheld
  · obtain ⟨c, he⟩ := advanceLock_shape h
    exact absurd he (by simp)
  · obtain ⟨w, q, share, hc⟩ := v1Seg_shape h
    rcases hc with he | he <;> exact absurd he (by simp)
  · obtain ⟨w, q, he⟩ := v2Seg_shape h
    exact absurd he (by simp)
  · obtain ⟨q, he⟩ := pSeg_shape h
    exact absurd he (by simp)

end Impl
end NewProtocol
