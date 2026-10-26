module

public import NewProtocolImpl.Conformance

/-!
# Checking that a trace's node acts promptly

`NewProtocol.Prompt` asks that whatever a node owes after a step, it discharges, or
stops owing, by a step at most `δ` later. A recorded trace is a prefix of a run
with a time for each step, so this can be decided on the steps it has, except near
its end: an obligation whose deadline is after the recording stopped could still be
discharged by a step the recording did not see (`PromptUntil`).

A recorded step can grow after it is taken. An implementation acting on an input
the specification has no counterpart for, storage confirming a write for one, emits
outputs the recorder adds to the last step, since nothing the specification reads
happened in between. So the check reads a run as moments rather than steps
(`moments`): each step with its own outputs at its time, then with each release
added at that release's time. Read as steps, a vote held back for storage would
count as cast when its proposal arrived.

What a node owes at a moment is decided over the obligations its history could give
rise to (`NewProtocolImpl.candidates`). Those are all of them
(`NewProtocolImpl.mem_candidates`), so a trace that passes owes nothing the check
did not look at (`checkPrompt_sound`).
-/

@[expose] public section

namespace NewProtocolDiff

open NewProtocol

deriving instance DecidableEq for Obligation

/-- A point in a recorded run: the node's history then, and when it was. -/
structure Moment where
  /-- The step the moment is in. -/
  step : Nat
  history : History
  time : Nat

variable (cfg : Config) (leader : EpochNumber → ViewNumber → Option PubKey) (node : PubKey)

/--
`NewProtocol.Prompt`, on the moments of a prefix of a run that stopped at `ended`.

Every obligation owed at a moment stops being owed by a moment at most `δ` later,
or the recording stopped no later than that, so a moment it did not see could
still discharge it.
-/
def PromptUntil (ms : List Moment) (ended δ : Nat) : Prop :=
  ∀ k (hk : k < ms.length), ∀ o, Owed cfg leader node ms[k].history o →
    (∃ j, ∃ hj : j < ms.length, k ≤ j ∧ ms[j].time ≤ ms[k].time + δ
      ∧ ¬ Owed cfg leader node ms[j].history o)
      ∨ ended ≤ ms[k].time + δ

/--
The moments of a recorded run.

Step `n` is taken at `times[n]` with its own outputs, the first of its outputs.
Each entry of `releases[n]` is a later time and how many outputs it added, in the
order they follow.
-/
def moments (h : History) (times : Array Nat) (releases : Array (List (Nat × Nat))) :
    List Moment :=
  h.zipIdx.flatMap fun (st, n) =>
    let rel := releases.getD n []
    let own := st.output.length - (rel.map (·.2)).sum
    let grown := rel.zipIdx.map fun ((t, _), j) => (t, own + ((rel.take (j + 1)).map (·.2)).sum)
    ((times.getD n 0, own) :: grown).map fun (t, c) =>
      ⟨n, h.take n ++ [⟨st.input, st.output.take c⟩], t⟩

/-- What the node owes at each moment. -/
def owedAt (ms : List Moment) : Array (List Obligation) :=
  Array.ofFn fun (k : Fin ms.length) =>
    (NewProtocolImpl.candidates cfg ms[k].history).filter fun o =>
      decide (Owed cfg leader node ms[k].history o)

/-- When moment `k` was, or 0 past the last. -/
def timeAt (ms : List Moment) (k : Nat) : Nat := (ms[k]?.map (·.time)).getD 0

/--
Whether `o` is not owed at some moment from `m` on, before the times pass
`deadline`.

The search stops at the first moment after the deadline. Times do not decrease,
so no later moment could be within it.
-/
def stopsBy (owed : Array (List Obligation)) (time : Nat → Nat) (o : Obligation)
    (deadline : Nat) (m : Nat) : Nat → Bool
  | 0 => false
  | fuel + 1 => decide (m < owed.size) && decide (time m ≤ deadline)
      && (!decide (o ∈ owed.getD m []) || stopsBy owed time o deadline (m + 1) fuel)

/--
Each obligation owed at a moment and still owed at every moment up to `δ` later,
with the moment's index, while the recording ran past that deadline.
-/
def promptFaults (ms : List Moment) (ended δ : Nat) : List (Nat × Obligation) :=
  let owed := owedAt cfg leader node ms
  (List.range ms.length).flatMap fun k =>
    let t := timeAt ms k
    if ended ≤ t + δ then []
    else ((owed.getD k []).filter fun o => !stopsBy owed (timeAt ms) o (t + δ) k ms.length).map (k, ·)

/-! ## Soundness -/

variable {cfg leader node}

theorem owedAt_size (ms : List Moment) : (owedAt cfg leader node ms).size = ms.length := by
  simp [owedAt]

theorem mem_owedAt {ms : List Moment} {k : Nat} (hk : k < ms.length) {o : Obligation} :
    o ∈ (owedAt cfg leader node ms).getD k [] ↔ Owed cfg leader node ms[k].history o := by
  simp only [owedAt, Array.getD_eq_getD_getElem?, Array.getElem?_ofFn, hk, dite_true,
    Option.getD_some, List.mem_filter, decide_eq_true_eq]
  exact ⟨And.right, fun ho => ⟨NewProtocolImpl.mem_candidates rfl ho, ho⟩⟩

theorem timeAt_of {ms : List Moment} {k : Nat} (hk : k < ms.length) : timeAt ms k = ms[k].time := by
  simp [timeAt, hk]

theorem stopsBy_witness {owed : Array (List Obligation)} {time : Nat → Nat} {o : Obligation}
    {deadline : Nat} : ∀ {m fuel}, stopsBy owed time o deadline m fuel = true →
      ∃ k, m ≤ k ∧ k < owed.size ∧ time k ≤ deadline ∧ o ∉ owed.getD k []
  | _, 0, hs => by simp [stopsBy] at hs
  | m, fuel + 1, hs => by
    simp only [stopsBy, Bool.and_eq_true, Bool.or_eq_true, Bool.not_eq_true',
      decide_eq_true_eq, decide_eq_false_iff_not] at hs
    obtain ⟨⟨hm, ht⟩, hno | hrest⟩ := hs
    · exact ⟨m, Nat.le_refl m, hm, ht, hno⟩
    · obtain ⟨k, hk, rest⟩ := stopsBy_witness hrest
      exact ⟨k, Nat.le_of_succ_le hk, rest⟩

/--
**A trace with no prompt fault obeys `PromptUntil`**: everything its node owed, it
discharged within `δ`, or the recording stopped before the deadline.
-/
theorem checkPrompt_sound {ms : List Moment} {ended δ : Nat}
    (hp : promptFaults cfg leader node ms ended δ = []) :
    PromptUntil cfg leader node ms ended δ := by
  intro k hk o ho
  rw [← timeAt_of hk]
  by_cases he : ended ≤ timeAt ms k + δ
  · exact Or.inr he
  have hnone := List.flatMap_eq_nil_iff.mp hp k (List.mem_range.mpr hk)
  simp only [he, ite_false, List.map_eq_nil_iff, List.filter_eq_nil_iff, Bool.not_eq_true',
    Bool.not_eq_false] at hnone
  obtain ⟨j, hj, hjs, ht, hno⟩ := stopsBy_witness (hnone o ((mem_owedAt hk).mpr ho))
  rw [owedAt_size] at hjs
  exact Or.inl ⟨j, hjs, hj, timeAt_of hjs ▸ ht, fun hj' => hno ((mem_owedAt hjs).mpr hj')⟩

end NewProtocolDiff
