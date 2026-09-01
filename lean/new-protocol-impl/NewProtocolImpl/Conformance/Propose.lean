module

public import NewProtocolImpl.Conformance.Vote1

/-!
# Proposing

Everything `StepSpec` asks about an emitted proposal.

`ProposalJustification` pins a proposal down once its view is fixed: the parent
certificate is the lock (after a timeout) or the certificate of the preceding
view, that certificate names the parent block we hold, and the headers content
supplies the header. The two candidates of `Impl.State.timeoutCandidate`
and `Impl.State.normalCandidate` are exactly those two readings, so relating
each to the rule is all this file does beyond the usual transfer.

The lock the timeout reading names is the one the step ends with, since the
proposals are the last round and the lock stopped moving at `Impl.st2`.
-/

@[expose] public section

set_option linter.unusedSectionVars false

open Std (TreeMap TreeSet)

namespace NewProtocol
namespace Impl

variable {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
variable {settled : TreeSet ViewNumber} {v : ViewNumber} {s t : State}

/-! ## What a candidate is

Each candidate carries its own justification: which certificate it extends, that
we hold the block that certificate names, and which header is headers.
-/

theorem parentMatches_spec {pcert : Cert1} {parent : Proposal}
    (h : parentMatches pcert parent = true) :
    pcert.view ≠ ViewNumber.genesis → blockHash parent = pcert.data.blockHash := by
  intro hne
  rw [parentMatches, Bool.or_eq_true] at h
  rcases h with hgen | hbh
  · exact absurd (by simpa using hgen) hne
  · simpa using hbh

theorem timeoutCandidate_spec {p : Proposal} (h : s.timeoutCandidate cfg v = some p) :
    p.viewNumber = v ∧ p.parentCert.view < v
      ∧ p.epoch = epochOf p.blockHeader.blockNumber cfg.epochHeight
      ∧ (∃ tc, p.timeoutEvidence = some tc ∧ s.timeoutCerts.get? v = some tc)
      ∧ s.lockedCert = some p.parentCert
      ∧ ∃ parent, s.proposals.get? p.parentCert.view = some parent
          ∧ (p.parentCert.view ≠ ViewNumber.genesis →
              blockHash parent = p.parentCert.data.blockHash)
          ∧ s.headers.get? (v, blockHash parent) = some p.blockHeader := by
  unfold State.timeoutCandidate at h
  simp only [Option.bind_eq_some_iff] at h
  obtain ⟨tc, htc, pcert, hlock, h⟩ := h
  split at h
  · rename_i hlt
    simp only [Option.bind_eq_some_iff] at h
    obtain ⟨parent, hpar, h⟩ := h
    split at h
    · rename_i hmatch
      simp only [Option.map_eq_some_iff] at h
      obtain ⟨hd, hhd, rfl⟩ := h
      exact ⟨rfl, hlt, rfl, ⟨tc, rfl, htc⟩, hlock, parent, hpar,
        parentMatches_spec hmatch, hhd⟩
    · exact absurd h (by simp)
  · exact absurd h (by simp)

theorem normalCandidate_spec {p : Proposal} (h : s.normalCandidate cfg v = some p) :
    p.viewNumber = v ∧ p.timeoutEvidence = none ∧ p.parentCert.view + 1 = v
      ∧ p.epoch = epochOf p.blockHeader.blockNumber cfg.epochHeight
      ∧ s.cert1s.get? (v - 1) = some p.parentCert
      ∧ ∃ parent, s.proposals.get? p.parentCert.view = some parent
          ∧ (p.parentCert.view ≠ ViewNumber.genesis →
              blockHash parent = p.parentCert.data.blockHash)
          ∧ s.headers.get? (v, blockHash parent) = some p.blockHeader := by
  unfold State.normalCandidate at h
  simp only [Option.bind_eq_some_iff] at h
  obtain ⟨pcert, hc1, h⟩ := h
  split at h
  · rename_i hsucc
    simp only [Option.bind_eq_some_iff] at h
    obtain ⟨parent, hpar, h⟩ := h
    split at h
    · rename_i hmatch
      simp only [Option.map_eq_some_iff] at h
      obtain ⟨hd, hhd, rfl⟩ := h
      exact ⟨rfl, rfl, hsucc, rfl, hc1, parent, hpar, parentMatches_spec hmatch, hhd⟩
    · exact absurd h (by simp)
  · exact absurd h (by simp)

/-! ## The obligation -/

theorem pSeg_frozen (f : StepFn) (hf : f ∈ pSeg cfg leader node t) (u : State) :
    (f u).1.lockedCert = u.lockedCert := by
  obtain ⟨k, -, rfl⟩ :=
    List.mem_map.mp
      (show f ∈ List.map (fun k => tryPropose cfg leader node k.1) t.headers.keys from hf)
  exact tryPropose_lock u

theorem pSeg_grows' (f : StepFn) (hf : f ∈ pSeg cfg leader node t) : Grows cfg f := by
  obtain ⟨k, -, rfl⟩ :=
    List.mem_map.mp
      (show f ∈ List.map (fun k => tryPropose cfg leader node k.1) t.headers.keys from hf)
  exact tryPropose_grows

/--
A proposal in a pass's output satisfies the proposing rule, at the state the step
ends in.
-/
theorem pass_propose (hwf : WF cfg t) {p : Proposal}
    (h : Output.send (.proposal p) ∈ (seq (rounds cfg leader node t) t).2) :
    leader p.viewNumber = some node
      ∧ ProposalWellFormed cfg p
      ∧ (match p.timeoutEvidence with
         | some tc => t.timeoutCerts.get? p.viewNumber = some tc
             ∧ (st5 cfg leader node t).lockedCert = some p.parentCert
         | none => t.cert1s.get? (p.viewNumber - 1) = some p.parentCert
             ∧ p.parentCert.view + 1 = p.viewNumber)
      ∧ (∃ parent, t.proposals.get? p.parentCert.view = some parent
          ∧ (p.parentCert.view ≠ ViewNumber.genesis →
              blockHash parent = p.parentCert.data.blockHash)
          ∧ t.headers.get? (p.viewNumber, blockHash parent) = some p.blockHeader)
      ∧ p.viewNumber ∉ t.proposedViews
      ∧ p.viewNumber ∈ (st5 cfg leader node t).proposedViews
      ∧ t.timeoutView < p.viewNumber ∧ t.barredView < p.viewNumber := by
  rw [pass_out] at h
  simp only [List.mem_append] at h
  -- Only the proposing round emits a proposal.
  rcases h with ((((h | h) | h) | h) | h)
  · obtain ⟨chain, c1, c2, he⟩ := dSeg_shape h
    exact absurd he (by simp)
  · obtain ⟨c, he⟩ := advanceLock_shape h
    exact absurd he (by simp)
  · obtain ⟨w, q, share, hc⟩ := v1Seg_shape h
    rcases hc with he | he <;> exact absurd he (by simp)
  · obtain ⟨w, q, he⟩ := v2Seg_shape h
    exact absurd he (by simp)
  · -- the proposing round
    obtain ⟨f, hf, u, hwfu, hfr, hle, hlockU', ho, -, hle', -⟩ :=
      mem_seq State.lockedCert (fun f hf => pSeg_grows' f hf) (fun f hf u => pSeg_frozen f hf u)
        (st4_stage hwf).2.2 h
    obtain ⟨k, -, rfl⟩ :=
      List.mem_map.mp
        (show f ∈ List.map (fun k => tryPropose cfg leader node k.1) t.headers.keys from hf)
    rcases tryPropose_cases (r := tryPropose cfg leader node k.1 u) rfl with heq |
      ⟨q, hcand, hto, hbar, hprop, hlead, heq⟩
    · rw [heq] at ho; exact absurd ho (by simp)
    · obtain rfl : p = q := by rw [heq] at ho; simpa using ho
      have hft : Frame t u := (st4_stage hwf).1.trans hfr
      have hlet : Le t u := (st4_stage hwf).2.1.trans hle
      have hlockU : u.lockedCert = (st5 cfg leader node t).lockedCert := by
        rw [st5_lock, ← st4_lock]; exact hlockU'
      -- the view proposed in is the view scanned, whichever candidate was taken
      have hmark : p.viewNumber ∈ (st5 cfg leader node t).proposedViews := by
        refine hle'.proposed _ ?_
        rw [heq]
        rcases Option.or_eq_some_iff.mp hcand with hc | ⟨-, hc⟩
        · rw [(timeoutCandidate_spec hc).1]; exact mem_insert_self
        · rw [(normalCandidate_spec hc).1]; exact mem_insert_self
      -- and the guards are about that view too
      rcases Option.or_eq_some_iff.mp hcand with hc | ⟨hnone, hc⟩
      · obtain ⟨hv, hlt, hep, ⟨tc, hte, htc⟩, hlock, parent, hpar, hmatch, hhd⟩ :=
          timeoutCandidate_spec hc
        rw [← hv] at hlt htc hhd hto hbar hprop hlead
        refine ⟨hlead, ⟨hlt, Or.inr ⟨tc, hte, ?_⟩, hep⟩, ?_, ⟨parent, ?_, hmatch, ?_⟩, ?_, hmark,
          ?_, ?_⟩
        · exact hwfu.timeoutCerts _ _ htc
        · rw [hte]
          exact ⟨by rw [← hft.timeoutCerts]; exact htc, by rw [← hlockU]; exact hlock⟩
        · rw [← hft.proposals]; exact hpar
        · rw [← hft.headers]; exact hhd
        · exact fun hcc => hprop (hlet.proposed _ hcc)
        · rw [← hft.timeoutView]; exact hto
        · rw [← hft.barredView]; exact hbar
      · obtain ⟨hv, hte, hsucc, hep, hc1, parent, hpar, hmatch, hhd⟩ := normalCandidate_spec hc
        rw [← hv] at hsucc hc1 hhd hto hbar hprop hlead
        refine ⟨hlead, ⟨?_, Or.inl hsucc, hep⟩, ?_, ⟨parent, ?_, hmatch, ?_⟩, ?_, hmark, ?_, ?_⟩
        · rw [← hsucc]; exact Nat.lt_succ_self _
        · rw [hte]
          exact ⟨by rw [← hft.cert1s]; exact hc1, hsucc⟩
        · rw [← hft.proposals]; exact hpar
        · rw [← hft.headers]; exact hhd
        · exact fun hcc => hprop (hlet.proposed _ hcc)
        · rw [← hft.timeoutView]; exact hto
        · rw [← hft.barredView]; exact hbar

end Impl
end NewProtocol
