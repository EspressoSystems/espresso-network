module

public import NewProtocolImpl.Conformance.Lock

/-!
# Marks

The five obligations that run the other way: `StepSpec.vote1Marked`,
`vote2Marked`, `proposedMarked`, `decidedMarked` and `vote1BranchesSound`. Each
says a mark cannot appear in silence — if the step marked a view voted, proposed
or decided, it emitted the action that does so.

They are the reason a node cannot conform by quietly retiring an opportunity, and
they are proved the same way each time: `Impl.seq_flip` finds the round the
mark flipped at, the field-preservation lemmas of
`NewProtocolImpl.Conformance.Fields` rule out the four rounds that do not touch
that mark, and the remaining round's `*_cases` equation says it emitted.
-/

@[expose] public section

set_option linter.unusedSectionVars false

open Std (TreeMap TreeSet)

namespace NewProtocol
namespace Impl

variable {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
variable {settled : TreeSet ViewNumber} {t : State} {v : ViewNumber}

/-- Every round of a pass grows the state. -/
theorem rounds_grows' (f : StepFn) (hf : f ∈ rounds cfg leader node t) : Grows cfg f := by
  rw [rounds_eq] at hf
  simp only [List.mem_append, List.mem_singleton] at hf
  rcases hf with (((h | h) | h) | h) | h
  · obtain ⟨v, -, rfl⟩ :=
      List.mem_map.mp (show f ∈ List.map (tryDecide cfg t.decidedViews) t.cert2s.keys from h)
    exact tryDecide_grows
  · rw [h]; exact advanceLock_grows
  · obtain ⟨v, -, rfl⟩ :=
      List.mem_map.mp (show f ∈ List.map (tryVote1 node) t.admitted.keys from h)
    exact tryVote1_grows
  · obtain ⟨v, -, rfl⟩ :=
      List.mem_map.mp (show f ∈ List.map (tryVote2 cfg node) t.admitted.keys from h)
    exact tryVote2_grows
  · obtain ⟨k, -, rfl⟩ :=
      List.mem_map.mp
        (show f ∈ List.map (fun k => tryPropose cfg leader node k.1) t.headers.keys from h)
    exact tryPropose_grows

/-!
## The obligations

Each proof isolates the acting round with `Impl.seq_flip`, then reads its
output off its `*_cases` equation. The four rounds that leave the mark alone are
dismissed by rewriting with their preservation lemma, which turns "the mark
appeared here" into "it was already there".
-/

theorem pass_vote1Marked (hwf : WF cfg t) (h0 : v ∉ t.voted1Views)
    (h1 : v ∈ (st5 cfg leader node t).voted1Views) :
    ∃ vt : Vote1, Output.send (.vote1 vt) ∈ (seq (rounds cfg leader node t) t).2
      ∧ vt.view = v := by
  obtain ⟨f, hf, u, -, -, -, hnp, hpp, hall⟩ :=
    seq_flip (P := fun s => v ∈ s.voted1Views) (fun f hf => rounds_grows' f hf) hwf h0
      (by rw [pass_state]; exact h1)
  rw [rounds_eq] at hf
  simp only [List.mem_append, List.mem_singleton] at hf
  rcases hf with (((hm | hm) | hm) | hm) | hm
  · obtain ⟨w, -, rfl⟩ := List.mem_map.mp
      (show f ∈ List.map (tryDecide cfg t.decidedViews) t.cert2s.keys from hm)
    rw [tryDecide_voted1] at hpp
    exact absurd hpp hnp
  · rw [hm, advanceLock_voted1] at hpp; exact absurd hpp hnp
  · obtain ⟨w, -, rfl⟩ :=
      List.mem_map.mp (show f ∈ List.map (tryVote1 node) t.admitted.keys from hm)
    rcases tryVote1_cases (r := tryVote1 node w u) rfl with heq |
      ⟨p, share, -, -, -, -, -, -, -, -, heq⟩
    · rw [heq] at hpp; exact absurd hpp hnp
    · refine ⟨⟨⟨blockHash p⟩, w, node⟩, hall _ ?_, ?_⟩
      · rw [heq]; exact List.mem_cons_self ..
      · -- the mark that appeared is the view this round was scanning
        rw [heq] at hpp
        rcases mem_insert.mp hpp with rfl | hc
        · rfl
        · exact absurd hc hnp
  · obtain ⟨w, -, rfl⟩ :=
      List.mem_map.mp (show f ∈ List.map (tryVote2 cfg node) t.admitted.keys from hm)
    rw [tryVote2_voted1] at hpp; exact absurd hpp hnp
  · obtain ⟨k, -, rfl⟩ :=
      List.mem_map.mp
        (show f ∈ List.map (fun k => tryPropose cfg leader node k.1) t.headers.keys from hm)
    rw [tryPropose_voted1] at hpp; exact absurd hpp hnp

theorem pass_vote2Marked (hwf : WF cfg t) (h0 : v ∉ t.voted2Views)
    (h1 : v ∈ (st5 cfg leader node t).voted2Views) :
    ∃ vt : Vote2, Output.send (.vote2 vt) ∈ (seq (rounds cfg leader node t) t).2
      ∧ vt.view = v := by
  obtain ⟨f, hf, u, -, -, -, hnp, hpp, hall⟩ :=
    seq_flip (P := fun s => v ∈ s.voted2Views) (fun f hf => rounds_grows' f hf) hwf h0
      (by rw [pass_state]; exact h1)
  rw [rounds_eq] at hf
  simp only [List.mem_append, List.mem_singleton] at hf
  rcases hf with (((hm | hm) | hm) | hm) | hm
  · obtain ⟨w, -, rfl⟩ := List.mem_map.mp
      (show f ∈ List.map (tryDecide cfg t.decidedViews) t.cert2s.keys from hm)
    rw [tryDecide_voted2] at hpp
    exact absurd hpp hnp
  · rw [hm, advanceLock_voted2] at hpp; exact absurd hpp hnp
  · obtain ⟨w, -, rfl⟩ :=
      List.mem_map.mp (show f ∈ List.map (tryVote1 node) t.admitted.keys from hm)
    rw [tryVote1_voted2] at hpp; exact absurd hpp hnp
  · obtain ⟨w, -, rfl⟩ :=
      List.mem_map.mp (show f ∈ List.map (tryVote2 cfg node) t.admitted.keys from hm)
    rcases tryVote2_cases (r := tryVote2 cfg node w u) rfl with heq |
      ⟨p, c, -, -, -, -, -, -, -, -, -, heq⟩
    · rw [heq] at hpp; exact absurd hpp hnp
    · refine ⟨⟨⟨blockHash p⟩, w, node⟩, hall _ ?_, ?_⟩
      · rw [heq]; exact List.mem_cons_self ..
      · rw [heq] at hpp
        rcases mem_insert.mp hpp with rfl | hc
        · rfl
        · exact absurd hc hnp
  · obtain ⟨k, -, rfl⟩ :=
      List.mem_map.mp
        (show f ∈ List.map (fun k => tryPropose cfg leader node k.1) t.headers.keys from hm)
    rw [tryPropose_voted2] at hpp; exact absurd hpp hnp

theorem pass_proposedMarked (hwf : WF cfg t) (h0 : v ∉ t.proposedViews)
    (h1 : v ∈ (st5 cfg leader node t).proposedViews) :
    ∃ p : Proposal, Output.send (.proposal p) ∈ (seq (rounds cfg leader node t) t).2
      ∧ p.viewNumber = v := by
  obtain ⟨f, hf, u, -, -, -, hnp, hpp, hall⟩ :=
    seq_flip (P := fun s => v ∈ s.proposedViews) (fun f hf => rounds_grows' f hf) hwf h0
      (by rw [pass_state]; exact h1)
  rw [rounds_eq] at hf
  simp only [List.mem_append, List.mem_singleton] at hf
  rcases hf with (((hm | hm) | hm) | hm) | hm
  · obtain ⟨w, -, rfl⟩ := List.mem_map.mp
      (show f ∈ List.map (tryDecide cfg t.decidedViews) t.cert2s.keys from hm)
    rw [tryDecide_proposed] at hpp
    exact absurd hpp hnp
  · rw [hm, advanceLock_proposed] at hpp; exact absurd hpp hnp
  · obtain ⟨w, -, rfl⟩ :=
      List.mem_map.mp (show f ∈ List.map (tryVote1 node) t.admitted.keys from hm)
    rw [tryVote1_proposed] at hpp; exact absurd hpp hnp
  · obtain ⟨w, -, rfl⟩ :=
      List.mem_map.mp (show f ∈ List.map (tryVote2 cfg node) t.admitted.keys from hm)
    rw [tryVote2_proposed] at hpp; exact absurd hpp hnp
  · obtain ⟨k, -, rfl⟩ :=
      List.mem_map.mp
        (show f ∈ List.map (fun k => tryPropose cfg leader node k.1) t.headers.keys from hm)
    rcases tryPropose_cases (r := tryPropose cfg leader node k.1 u) rfl with heq |
      ⟨p, hcand, -, -, -, -, heq⟩
    · rw [heq] at hpp; exact absurd hpp hnp
    · refine ⟨p, hall _ ?_, ?_⟩
      · rw [heq]; exact List.mem_cons_self ..
      · rw [heq] at hpp
        rcases mem_insert.mp hpp with rfl | hc
        · rcases Option.or_eq_some_iff.mp hcand with hc | ⟨-, hc⟩
          · exact (timeoutCandidate_spec hc).1
          · exact (normalCandidate_spec hc).1
        · exact absurd hc hnp

theorem pass_decidedMarked (hwf : WF cfg t) (h0 : v ∉ t.decidedViews)
    (h1 : v ∈ (st5 cfg leader node t).decidedViews) :
    ∃ chain c1 c2 b, Output.decided chain c1 c2 ∈ (seq (rounds cfg leader node t) t).2
      ∧ b ∈ chain ∧ b.viewNumber = v := by
  obtain ⟨f, hf, u, hwfu, -, -, hnp, hpp, hall⟩ :=
    seq_flip (P := fun s => v ∈ s.decidedViews) (fun f hf => rounds_grows' f hf) hwf h0
      (by rw [pass_state]; exact h1)
  rw [rounds_eq] at hf
  simp only [List.mem_append, List.mem_singleton] at hf
  rcases hf with (((hm | hm) | hm) | hm) | hm
  · obtain ⟨w, -, rfl⟩ := List.mem_map.mp
      (show f ∈ List.map (tryDecide cfg t.decidedViews) t.cert2s.keys from hm)
    rcases tryDecide_cases (r := tryDecide cfg t.decidedViews w u) rfl with heq |
      ⟨p, c1, c2, chain, -, -, -, -, -, -, -, heq⟩
    · rw [heq] at hpp; exact absurd hpp hnp
    · rw [heq] at hpp
      rcases mem_decideFold_cases hpp with hc | ⟨b, hb, hbv⟩
      · exact absurd hc hnp
      · exact ⟨chain, c1, c2, b, hall _ (by rw [heq]; exact List.mem_cons_self ..), hb, hbv⟩
  · rw [hm, advanceLock_decided] at hpp; exact absurd hpp hnp
  · obtain ⟨w, -, rfl⟩ :=
      List.mem_map.mp (show f ∈ List.map (tryVote1 node) t.admitted.keys from hm)
    rw [tryVote1_decided] at hpp; exact absurd hpp hnp
  · obtain ⟨w, -, rfl⟩ :=
      List.mem_map.mp (show f ∈ List.map (tryVote2 cfg node) t.admitted.keys from hm)
    rw [tryVote2_decided] at hpp; exact absurd hpp hnp
  · obtain ⟨k, -, rfl⟩ :=
      List.mem_map.mp
        (show f ∈ List.map (fun k => tryPropose cfg leader node k.1) t.headers.keys from hm)
    rw [tryPropose_decided] at hpp; exact absurd hpp hnp

/-- A branch record only ever appears together with the vote1 that endorsed it. -/
theorem pass_branchesSound (hwf : WF cfg t) {u : ViewNumber} (h0 : t.vote1Branches.get? v = none)
    (h1 : (st5 cfg leader node t).vote1Branches.get? v = some u) :
    ∃ vt : Vote1, Output.send (.vote1 vt) ∈ (seq (rounds cfg leader node t) t).2
      ∧ vt.view = v := by
  obtain ⟨f, hf, w, -, -, -, hnp, hpp, hall⟩ :=
    seq_flip (P := fun s => (s.vote1Branches.get? v).isSome = true)
      (fun f hf => rounds_grows' f hf) hwf (by rw [h0]; simp)
      (by rw [pass_state, h1]; rfl)
  rw [rounds_eq] at hf
  simp only [List.mem_append, List.mem_singleton] at hf
  rcases hf with (((hm | hm) | hm) | hm) | hm
  · obtain ⟨w, -, rfl⟩ := List.mem_map.mp
      (show f ∈ List.map (tryDecide cfg t.decidedViews) t.cert2s.keys from hm)
    rw [tryDecide_branches] at hpp
    exact absurd hpp hnp
  · rw [hm, advanceLock_branches] at hpp; exact absurd hpp hnp
  · obtain ⟨x, -, rfl⟩ :=
      List.mem_map.mp (show f ∈ List.map (tryVote1 node) t.admitted.keys from hm)
    rcases tryVote1_cases (r := tryVote1 node x w) rfl with heq |
      ⟨p, share, -, -, -, -, -, -, -, -, heq⟩
    · rw [heq] at hpp; exact absurd hpp hnp
    · refine ⟨⟨⟨blockHash p⟩, x, node⟩, hall _ ?_, ?_⟩
      · rw [heq]; exact List.mem_cons_self ..
      · -- the record that appeared is at the view this round was scanning
        rw [heq] at hpp
        dsimp only at hpp
        by_cases hvx : v = x
        · exact hvx.symm
        · rw [get?_insert, if_neg hvx] at hpp
          exact absurd hpp hnp
  · obtain ⟨x, -, rfl⟩ :=
      List.mem_map.mp (show f ∈ List.map (tryVote2 cfg node) t.admitted.keys from hm)
    rw [tryVote2_branches] at hpp; exact absurd hpp hnp
  · obtain ⟨k, -, rfl⟩ :=
      List.mem_map.mp
        (show f ∈ List.map (fun k => tryPropose cfg leader node k.1) t.headers.keys from hm)
    rw [tryPropose_branches] at hpp; exact absurd hpp hnp

end Impl
end NewProtocol
