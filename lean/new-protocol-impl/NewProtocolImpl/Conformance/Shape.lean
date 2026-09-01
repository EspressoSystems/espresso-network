module

public import NewProtocolImpl.Conformance.Pass

/-!
# Which round emitted it

Each round emits only its own kind of output, so the message constructor
identifies the round — a vote1 can only have come from the vote1
round, and so on. The obligations about actions all start by using that, and the
four segments that *cannot* have emitted the output in hand are dismissed with
the corollaries here.

Nothing in this file says anything about *when* a round acts; that is
`NewProtocolImpl.Conformance.Cases`, which these are read off.
-/

@[expose] public section

set_option linter.unusedSectionVars false

open Std (TreeMap TreeSet)

namespace NewProtocol
namespace Impl

variable {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
variable {settled : TreeSet ViewNumber} {v : ViewNumber} {s u t : State} {o : Output}

/-! ## One round -/

theorem tryDecide_shape (h : o ∈ (tryDecide cfg settled v s).2) :
    ∃ chain c1 c2, o = Output.decided chain c1 c2 := by
  rcases tryDecide_cases rfl with heq | ⟨p, c1, c2, chain, -, -, -, -, -, -, -, heq⟩
  · rw [heq] at h; exact absurd h (by simp)
  · rw [heq] at h; exact ⟨chain, c1, c2, by simpa using h⟩

theorem advanceLock_shape (h : o ∈ (advanceLock s).2) :
    ∃ c, o = Output.send (.cert1 c) := by
  rcases advanceLock_cases rfl with heq | ⟨c, -, -, heq⟩
  · rw [heq] at h; exact absurd h (by simp)
  · rw [heq] at h; exact ⟨c, by simpa using h⟩

theorem tryVote1_shape (h : o ∈ (tryVote1 node v s).2) :
    ∃ p share, o = Output.send (.vote1 ⟨⟨blockHash p, p.epoch⟩, v, node⟩)
      ∨ o = Output.send (.vidShare share) := by
  rcases tryVote1_cases rfl with heq | ⟨p, share, -, -, -, -, -, -, -, -, heq⟩
  · rw [heq] at h; exact absurd h (by simp)
  · rw [heq] at h; exact ⟨p, share, by simpa [or_comm] using h⟩

theorem tryVote2_shape (h : o ∈ (tryVote2 cfg node v s).2) :
    ∃ p, o = Output.send (.vote2 ⟨⟨blockHash p, p.epoch⟩, v, node⟩) := by
  rcases tryVote2_cases rfl with heq | ⟨p, c, -, -, -, -, -, -, -, -, -, heq⟩
  · rw [heq] at h; exact absurd h (by simp)
  · rw [heq] at h; exact ⟨p, by simpa using h⟩

theorem tryPropose_shape (h : o ∈ (tryPropose cfg leader node v s).2) :
    ∃ p, o = Output.send (.proposal p) := by
  rcases tryPropose_cases rfl with heq | ⟨p, -, -, -, -, -, heq⟩
  · rw [heq] at h; exact absurd h (by simp)
  · rw [heq] at h; exact ⟨p, by simpa using h⟩

/-! ## A whole segment

A property of every round's outputs is a property of the segment's, whatever
state the segment starts from.
-/

theorem mem_seq_shape {fs : List StepFn} {P : Output → Prop}
    (hs : ∀ f ∈ fs, ∀ w, ∀ o ∈ (f w).2, P o) (h : o ∈ (seq fs u).2) : P o := by
  induction fs generalizing u with
  | nil => exact absurd h (by simp [seq])
  | cons f fs ih =>
    rcases List.mem_append.mp (show o ∈ (f u).2 ++ (seq fs (f u).1).2 from h) with h1 | h2
    · exact hs f (List.mem_cons_self ..) u o h1
    · exact ih (fun g hg => hs g (List.mem_cons_of_mem _ hg)) h2

theorem dSeg_shape (h : o ∈ (seq (dSeg cfg t) u).2) :
    ∃ chain c1 c2, o = Output.decided chain c1 c2 := by
  refine mem_seq_shape (P := fun o => ∃ chain c1 c2, o = Output.decided chain c1 c2)
    (fun f hf w o' ho' => ?_) h
  obtain ⟨v, -, rfl⟩ :=
    List.mem_map.mp (show f ∈ List.map (tryDecide cfg t.decidedViews) t.cert2s.keys from hf)
  exact tryDecide_shape ho'

theorem v1Seg_shape (h : o ∈ (seq (v1Seg node t) u).2) :
    ∃ w p share, o = Output.send (.vote1 ⟨⟨blockHash p, p.epoch⟩, w, node⟩)
      ∨ o = Output.send (.vidShare share) := by
  refine mem_seq_shape (P := fun o => ∃ w p share,
      o = Output.send (.vote1 ⟨⟨blockHash p, p.epoch⟩, w, node⟩) ∨ o = Output.send (.vidShare share))
    (fun f hf w o' ho' => ?_) h
  obtain ⟨v, -, rfl⟩ :=
    List.mem_map.mp (show f ∈ List.map (tryVote1 node) t.admitted.keys from hf)
  obtain ⟨p, share, ho⟩ := tryVote1_shape ho'
  exact ⟨v, p, share, ho⟩

theorem v2Seg_shape (h : o ∈ (seq (v2Seg cfg node t) u).2) :
    ∃ w p, o = Output.send (.vote2 ⟨⟨blockHash p, p.epoch⟩, w, node⟩) := by
  refine mem_seq_shape (P := fun o => ∃ w p, o = Output.send (.vote2 ⟨⟨blockHash p, p.epoch⟩, w, node⟩))
    (fun f hf w o' ho' => ?_) h
  obtain ⟨v, -, rfl⟩ :=
    List.mem_map.mp (show f ∈ List.map (tryVote2 cfg node) t.admitted.keys from hf)
  obtain ⟨p, ho⟩ := tryVote2_shape ho'
  exact ⟨v, p, ho⟩

theorem pSeg_shape (h : o ∈ (seq (pSeg cfg leader node t) u).2) :
    ∃ p, o = Output.send (.proposal p) := by
  refine mem_seq_shape (P := fun o => ∃ p, o = Output.send (.proposal p))
    (fun f hf w o' ho' => ?_) h
  obtain ⟨k, -, rfl⟩ :=
    List.mem_map.mp (show f ∈ List.map (fun k => tryPropose cfg leader node k.1) t.headers.keys from hf)
  exact tryPropose_shape ho'

end Impl
end NewProtocol
