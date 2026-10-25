module

public import NewProtocolImpl.Conformance

/-!
# Checking that a trace's node acts promptly

`NewProtocol.Prompt` asks that whatever a node owes after a step, it discharges, or
stops owing, by a step at most `δ` later, and that it answers every timer it takes
within the same bound. A recorded trace is a prefix of a run
with a time for each step, so this can be decided on the steps it has, except near
its end: an obligation or timer whose deadline is after the recording stopped could
still be discharged by a step the recording did not see (`PromptUntil`,
`TimersUntil`).

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
did not look at (`checkPrompt_sound`). The timers a node takes are read off the
steps' inputs, as `History.TimerAt` defines them (`checkTimers_sound`).
-/

@[expose] public section

namespace NewProtocolDiff

open NewProtocol NewProtocol.Lists NewProtocolImpl

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
`NewProtocol.Prompt.timers`, on the moments of a prefix of a run that stopped at
`ended`.

Every timer the node took, it answers by a moment at most `δ` later, or by then it
has grounds for a later view, or the recording stopped no later than that.
-/
def TimersUntil (ms : List Moment) (ended δ : Nat) : Prop :=
  ∀ k (hk : k < ms.length), ∀ v, ms[k].history.TimerAt cfg ms[k].step v →
    (∃ j, ∃ hj : j < ms.length, k ≤ j ∧ ms[j].time ≤ ms[k].time + δ
      ∧ (ms[j].history.TimerAnswered cfg node ms[k].step v
        ∨ ∃ u, ms[j].history.ViewGround cfg u ∧ v < u))
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

/--
Whether `h` answers the timer for `v` taken in step `n`, or has grounds past `v`.

The epoch the vote names is the one the node was in at the timer.
-/
def timerSettled (h : History) (n : Nat) (v : ViewNumber) : Bool :=
  let e := epochOfHistory cfg (h.take n)
  ((h.drop n).any fun st => (timeoutVotesOf st).any fun tv =>
      decide (tv.view = v ∧ tv.signer = node ∧ tv.data.epoch = e))
    || decide (v < viewOf cfg h)

/--
Whether the timer for `v` taken in step `n` is settled at some moment from `m` on,
before the times pass `deadline`.
-/
def settlesBy (ms : List Moment) (n : Nat) (v : ViewNumber) (deadline : Nat) (m : Nat) : Nat → Bool
  | 0 => false
  | fuel + 1 =>
    match ms[m]? with
    | some x => decide (x.time ≤ deadline)
        && (timerSettled cfg node x.history n v || settlesBy ms n v deadline (m + 1) fuel)
    | none => false

/--
Each timer taken at a moment and neither answered nor overtaken at any moment up
to `δ` later, with the moment's index, while the recording ran past that deadline.
-/
def timerFaults (ms : List Moment) (ended δ : Nat) : List (Nat × ViewNumber) :=
  (List.range ms.length).filterMap fun k =>
    match ms[k]? with
    | some x =>
      match timerView cfg x.history x.step with
      | some v =>
        if ended ≤ x.time + δ || settlesBy cfg node ms x.step v (x.time + δ) k ms.length then none
        else some (k, v)
      | none => none
    | none => none

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

theorem timerSettled_of {h : History} {n : Nat} {v : ViewNumber} (hs : timerSettled cfg node h n v = true) :
    h.TimerAnswered cfg node n v ∨ ∃ u, h.ViewGround cfg u ∧ v < u := by
  simp only [timerSettled, Bool.or_eq_true, List.any_eq_true, decide_eq_true_eq] at hs
  rcases hs with ⟨st, hst, tv, htv, hview, hsig, hep⟩ | hlt
  · left
    obtain ⟨j, hj⟩ := List.mem_iff_getElem?.mp hst
    rw [List.getElem?_drop] at hj
    obtain ⟨⟨e, L⟩, w, k⟩ := tv
    simp only at hview hsig hep
    subst hview hsig hep
    exact ⟨_, L, n + j, Nat.le_add_right _ _, inEpoch_epochOfHistory cfg _, st, hj,
      mem_timeoutVotesOf_iff.mp htv⟩
  · exact Or.inr ⟨_, (inView_viewOf cfg h).1, hlt⟩

theorem settlesBy_witness {ms : List Moment} {n : Nat} {v : ViewNumber} {deadline : Nat} :
    ∀ {m fuel}, settlesBy cfg node ms n v deadline m fuel = true →
      ∃ j, ∃ hj : j < ms.length, m ≤ j ∧ ms[j].time ≤ deadline
        ∧ timerSettled cfg node ms[j].history n v = true
  | _, 0, hs => by simp [settlesBy] at hs
  | m, fuel + 1, hs => by
    unfold settlesBy at hs
    split at hs
    · rename_i x hx
      obtain ⟨hm, rfl⟩ := List.getElem?_eq_some_iff.mp hx
      simp only [Bool.and_eq_true, Bool.or_eq_true, decide_eq_true_eq] at hs
      obtain ⟨ht, hset | hrest⟩ := hs
      · exact ⟨m, hm, Nat.le_refl m, ht, hset⟩
      · obtain ⟨j, hj, hmj, rest⟩ := settlesBy_witness hrest
        exact ⟨j, hj, Nat.le_of_succ_le hmj, rest⟩
    · cases hs

/--
**A trace with no timer fault obeys `TimersUntil`**: every timer its node took, it
answered within `δ`, or had grounds past its view by then, or the recording stopped
before the deadline.
-/
theorem checkTimers_sound {ms : List Moment} {ended δ : Nat}
    (hp : timerFaults cfg node ms ended δ = []) : TimersUntil cfg node ms ended δ := by
  intro k hk v ht
  have hnone := List.filterMap_eq_nil_iff.mp hp k (List.mem_range.mpr hk)
  simp only [List.getElem?_eq_getElem hk, timerView_eq_some.mpr ht] at hnone
  by_cases he : ended ≤ ms[k].time + δ
  · exact Or.inr he
  · have hs : settlesBy cfg node ms ms[k].step v (ms[k].time + δ) k ms.length = true := by
      cases hsb : settlesBy cfg node ms ms[k].step v (ms[k].time + δ) k ms.length
      · simp [he, hsb] at hnone
      · rfl
    obtain ⟨j, hj, hkj, htj, hset⟩ := settlesBy_witness hs
    exact Or.inl ⟨j, hj, hkj, htj, timerSettled_of hset⟩

end NewProtocolDiff
