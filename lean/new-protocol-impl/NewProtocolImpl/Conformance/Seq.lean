module

public import NewProtocolImpl.Conformance.Rounds

/-!
# Sequencing a pass

Four facts about `Impl.seq`, and nothing about the rounds themselves.

The obligations of `StepSpec` that constrain actions quantify over the outputs of
a whole step, so each one has to be traced back to the round that emitted it and
the state that round ran at. `Impl.mem_seq` does the tracing; the two
`Frame`s and two `Le`s it returns are the transfer: content is the same at the
start, at the round and at the end, a mark absent at the round was absent at the
start, and one set by the round is still set at the end.

`Impl.seq_append` splits a pass at any point, which is how the *order* of the
rounds becomes usable, and `Impl.seq_proj` lifts a field no round of a segment
writes to the segment as a whole. `Impl.seq_at` is the counterpart of
`mem_seq` for a round that emitted nothing: where in the pass it ran.
-/

@[expose] public section

set_option linter.unusedSectionVars false

open Std (TreeMap TreeSet)

namespace NewProtocol
namespace Impl

variable {s : State}

/-- A pass splits at any point: the second half runs on what the first half left. -/
theorem seq_append (fs gs : List StepFn) (s : State) :
    seq (fs ++ gs) s = ((seq gs (seq fs s).1).1, (seq fs s).2 ++ (seq gs (seq fs s).1).2) := by
  induction fs generalizing s with
  | nil => rfl
  | cons f fs ih => simp only [List.cons_append, seq, ih, List.append_assoc]

/-- A field no round of a segment writes is the field the segment started with. -/
theorem seq_proj {α : Type} (proj : State → α) {fs : List StepFn}
    (h : ∀ f ∈ fs, ∀ t, proj (f t).1 = proj t) (s : State) : proj (seq fs s).1 = proj s := by
  induction fs generalizing s with
  | nil => rfl
  | cons f fs ih =>
    rw [show (seq (f :: fs) s).1 = (seq fs (f s).1).1 from rfl,
      ih (fun g hg => h g (List.mem_cons_of_mem _ hg)), h f (List.mem_cons_self ..)]

/--
An output of a pass comes from one of its rounds, at a state between the start
and the end.

`proj` is any field every round of the pass leaves alone; the conclusion says it
is the same at that state as at the start, which is what the lock and the branch
records need. The last clause says the round's *other* outputs reach the pass too
— what `StepSpec.vote1CarriesShare` needs, the share travelling with the vote.
-/
theorem mem_seq {α : Type} (proj : State → α) {fs : List StepFn} {o : Output}
    (hg : ∀ f ∈ fs, Grows f) (hp : ∀ f ∈ fs, ∀ u, proj (f u).1 = proj u) (hwf : WF s)
    (h : o ∈ (seq fs s).2) :
    ∃ f ∈ fs, ∃ t, WF t ∧ Frame s t ∧ Le s t ∧ proj t = proj s ∧ o ∈ (f t).2
      ∧ Frame (f t).1 (seq fs s).1 ∧ Le (f t).1 (seq fs s).1
      ∧ ∀ o' ∈ (f t).2, o' ∈ (seq fs s).2 := by
  induction fs generalizing s with
  | nil => exact absurd h (by simp [seq])
  | cons f fs ih =>
    have hf := hg f (List.mem_cons_self ..) s hwf
    have hrest := seq_grows (fun g hg' => hg g (List.mem_cons_of_mem _ hg')) (f s).1 hf.2.2
    rcases List.mem_append.mp (show o ∈ (f s).2 ++ (seq fs (f s).1).2 from h) with h1 | h2
    · exact ⟨f, List.mem_cons_self .., s, hwf, Frame.refl s, Le.refl s, rfl, h1,
        hrest.1, hrest.2.1, fun o' ho' => List.mem_append.mpr (Or.inl ho')⟩
    · obtain ⟨g, hgm, t, hwft, hfr, hle, hpr, ho, hfr', hle', hall⟩ :=
        ih (fun g hg' => hg g (List.mem_cons_of_mem _ hg'))
          (fun g hg' => hp g (List.mem_cons_of_mem _ hg')) hf.2.2 h2
      exact ⟨g, List.mem_cons_of_mem _ hgm, t, hwft, hf.1.trans hfr, hf.2.1.trans hle,
        hpr.trans (hp f (List.mem_cons_self ..) s), ho, hfr', hle',
        fun o' ho' => List.mem_append.mpr (Or.inr (hall o' ho'))⟩

/--
Where a round *is*, as opposed to where an output came from.

`Impl.mem_seq` traces an output back to the round that emitted it; this is
needed when the round emitted nothing and it is precisely its silence that has
to be explained.
-/
theorem seq_at {α : Type} (proj : State → α) : ∀ {fs : List StepFn}, (∀ f ∈ fs, Grows f) →
    (∀ f ∈ fs, ∀ w, proj (f w).1 = proj w) → ∀ {u0 : State}, WF u0 → ∀ f ∈ fs,
      ∃ u, WF u ∧ Frame u0 u ∧ Le u0 u ∧ proj u = proj u0
        ∧ Frame (f u).1 (seq fs u0).1 ∧ Le (f u).1 (seq fs u0).1
  | [], _, _, _, _, f, hf => absurd hf (by simp)
  | g :: gs, hg, hpj, u0, hwf, f, hf => by
    have hgg := hg g (List.mem_cons_self ..) u0 hwf
    rcases List.mem_cons.mp hf with hfg | hf'
    · rw [hfg]
      have hrest := seq_grows (fun x hx => hg x (List.mem_cons_of_mem _ hx)) (g u0).1 hgg.2.2
      exact ⟨u0, hwf, Frame.refl u0, Le.refl u0, rfl, hrest.1, hrest.2.1⟩
    · obtain ⟨u, hwfu, hfr, hle, hpu, hfr', hle'⟩ :=
        seq_at proj (fun x hx => hg x (List.mem_cons_of_mem _ hx))
          (fun x hx => hpj x (List.mem_cons_of_mem _ hx)) hgg.2.2 f hf'
      exact ⟨u, hwfu, hgg.1.trans hfr, hgg.2.1.trans hle,
        hpu.trans (hpj g (List.mem_cons_self ..) u0), hfr', hle'⟩

/--
Where a pass changed something: the first round at which a property went from
false to true, and the outputs of that round.

This is the direction the *mark* obligations need — `StepSpec.vote1Marked` and
friends, which say a mark only appears by emitting the action that sets it. Every
round grows the state, so the flip happens at exactly one round, and it is that
round which owes the output.
-/
theorem seq_flip {fs : List StepFn} {P : State → Prop} (hg : ∀ f ∈ fs, Grows f) (hwf : WF s)
    (h0 : ¬ P s) (h1 : P (seq fs s).1) :
    ∃ f ∈ fs, ∃ u, WF u ∧ Frame s u ∧ Le s u ∧ ¬ P u ∧ P (f u).1
      ∧ ∀ o ∈ (f u).2, o ∈ (seq fs s).2 := by
  induction fs generalizing s with
  | nil => exact absurd (show P s from h1) h0
  | cons f fs ih =>
    have hf := hg f (List.mem_cons_self ..) s hwf
    by_cases hp : P (f s).1
    · exact ⟨f, List.mem_cons_self .., s, hwf, Frame.refl s, Le.refl s, h0, hp,
        fun o ho => List.mem_append.mpr (Or.inl ho)⟩
    · obtain ⟨g, hgm, u, hwfu, hfr, hle, hnp, hpp, hall⟩ :=
        ih (fun g hg' => hg g (List.mem_cons_of_mem _ hg')) hf.2.2 hp h1
      exact ⟨g, List.mem_cons_of_mem _ hgm, u, hwfu, hf.1.trans hfr, hf.2.1.trans hle, hnp, hpp,
        fun o ho => List.mem_append.mpr (Or.inr (hall o ho))⟩

end Impl
end NewProtocol
