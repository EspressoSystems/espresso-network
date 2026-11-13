module

public import NewProtocolImpl.Conformance
public import NewProtocolDiff.Trace

/-!
# Checking that a trace's node acts in the step it owes

`NewProtocol.Prompt` asks that whatever a node owes after a step, it discharges,
or stops owing, by a step at most `δ` later, and that it answers every timer it
takes within the same bound. A recorded trace is checked for something read off
its steps alone, with no clock: what the node owes after a step, it acted on in
that step (`StepPrompt`), and a timer it takes, it answers in that step
(`TimersAnswered`). Consensus takes an input and emits what it draws at once, so a
node that obeys both discharges what it owes within the time a step takes, and
the storage writes it waits for.

A node acts on an obligation by meeting it, or by holding its action back until
storage confirms a write, which the trace says where it does (`heldThrough`).
Outputs the node emits on an input the specification has no counterpart for join
the step before. A certificate consensus took inside the message of the next
step is a step of its own, marked as taken with the next (`joined`): what it gives
rise to, the node acts on in the next.

Two things the steps alone would hide are checked on the dropped inputs
themselves. An action the node owed and acts on only at one has to be the release
of an action held back for the write the input confirms, or to have waited for
the DRB result or the built block the input is (`droppedActionFaults`). An action held back and still owed at the
end has to be waiting for a write storage did not confirm (`unreleasedIn`).

What a node owes after a step is decided over the obligations its history could
give rise to (`NewProtocolImpl.candidates`). Those are all of them
(`NewProtocolImpl.mem_candidates`), so a trace that passes owes nothing the check
did not look at (`checkSteps_sound`, `checkTimers_sound`).
-/

@[expose] public section

namespace NewProtocolDiff

open NewProtocol NewProtocol.Lists NewProtocolImpl

deriving instance DecidableEq for Obligation

variable (cfg : Config) (leader : EpochNumber → ViewNumber → Option PubKey) (node : PubKey)

/--
What an obligation's action is when the implementation holds it back for
storage, with its view and epoch: a vote1 and a re-vote's vote1 as `vote1`, a
vote2 as `vote2`, a proposal as `proposal`. A decide is never held back.
-/
def _root_.NewProtocol.Obligation.heldAs : Obligation → Option (HeldKind × ViewNumber × EpochNumber)
  | .vote1 p => some (.vote1, p.viewNumber, p.epoch)
  | .vote1Again r => some (.vote1, r.view, r.cert.data.epoch)
  | .vote2 c => some (.vote2, c.view, c.data.epoch)
  | .decide _ => none
  | .propose e v => some (.proposal, v, e)

/-- The first of the steps taken together with step `k`, which `joined` names. -/
def groupStart (joined : List Nat) (k : Nat) : Nat → Nat
  | 0 => k
  | fuel + 1 => if k > 0 && joined.contains (k - 1) then groupStart joined (k - 1) fuel else k

/--
Whether `held` notes, in a step at or before `m`, a timeout vote for the view
of `o` or a later one as held back.

A node that holds its timeout vote back has given the view up, and every view
before it, as one that sent the vote has (`History.TimedOut`): it votes and
proposes there no more.
-/
def gaveUp (held : List HeldAt) (o : Obligation) (m : Nat) : Bool :=
  held.any fun x =>
    decide (x.step ≤ m) && x.kind == .timeout && (o.heldAs.map (·.2.1)).any (decide <| · ≤ x.view)

/--
Whether `held` notes, in a step at or before `m`, the action of `o` as held
back, after which the node owed `o` after every step up to `m`, or a timeout
vote that gives the view of `o` up (`gaveUp`). `owes j o` is whether `o` is
owed after the first `j` steps.

A held-back action is released once storage confirms the write, or dropped once
the node gives the view up, but the node decided on it in the step that held it
back. One held back before the obligation last arose was another.
-/
def heldThrough (owes : Nat → Obligation → Bool) (held : List HeldAt)
    (o : Obligation) (m : Nat) : Bool :=
  held.any fun x =>
    decide (x.step ≤ m)
      && ((o.heldAs == some (x.kind, x.view, x.epoch)
          && (List.range' (x.step + 1) (m + 1 - x.step)).all fun j => owes j o)
        || (x.kind == .timeout && (o.heldAs.map (·.2.1)).any (decide <| · ≤ x.view)))

/--
`NewProtocol.Prompt`, read off the steps of a recorded run: whatever the node owes
after a step not taken with the next, it held back in a step since which it has
owed it throughout, or it has held back its timeout vote for the view or a later
one. Owing it otherwise means the node did not act on it in the step it was owed
in.
-/
def StepPrompt (h : History) (joined : List Nat) (held : List HeldAt) :
    Prop :=
  ∀ m, m < h.length → m ∉ joined → ∀ o, Owed cfg leader node (h.take (m + 1)) o →
    ∃ x, x ∈ held ∧ x.step ≤ m
      ∧ ((o.heldAs = some (x.kind, x.view, x.epoch)
          ∧ ∀ j, x.step + 1 ≤ j → j ≤ m + 1 → Owed cfg leader node (h.take j) o)
        ∨ (x.kind = .timeout ∧ ∃ u, o.heldAs.map (·.2.1) = some u ∧ u ≤ x.view))

/-- What the node owes after its first `j` steps, at index `j`. -/
def owedAfter (h : History) : Array (List Obligation) :=
  Array.ofFn fun (j : Fin (h.length + 1)) =>
    (candidates cfg (h.take j)).filter fun o => decide (Owed cfg leader node (h.take j) o)

/--
Each step of a run of `len` steps not taken with the next, with an obligation
owed after it that the node did not hold back (`heldThrough`), on what `owed`
says the node owes after each prefix.

What the node owes is decided once per step (`owedAfter`), so the check is
quadratic in the length of the run.
-/
def stepFaultsIn (owed : Array (List Obligation)) (len : Nat) (joined : List Nat)
    (held : List HeldAt) : List (Nat × Obligation) :=
  let owes := fun j o => (owed.getD j []).contains o
  (List.range len).flatMap fun m =>
    if joined.contains m then []
    else ((owed.getD (m + 1) []).filter fun o => !heldThrough owes held o m).map (m, ·)

/-- `stepFaultsIn` on what the node owes after each step of `h`. -/
def stepFaults (h : History) (joined : List Nat) (held : List HeldAt) :
    List (Nat × Obligation) :=
  stepFaultsIn (owedAfter cfg leader node h) h.length joined held

/--
The step `o`, owed after step `m`, last arose in: the latest at or before `m`
before which it was not owed, by what `owed` says.
-/
def aroseIn (owed : Array (List Obligation)) (o : Obligation) (m : Nat) : Nat :=
  ((List.range (m + 1)).reverse.find? fun k => !(owed.getD k []).contains o).getD 0

/-- Whether step `st` has the action `o` asks of `node`: its vote, proposal, re-vote request or decide. -/
def does (st : Step) : Obligation → Bool
  | .vote1 p => (vote1sOf st).any fun v => decide (v.view = p.viewNumber ∧ v.signer = node)
  | .vote1Again r => (vote1sOf st).any fun v => decide (v.view = r.view ∧ v.signer = node)
  | .vote2 c => (vote2sOf st).any fun v => decide (v.view = c.view ∧ v.signer = node)
  | .decide _ => !(decidesOf st).isEmpty
  | .propose e v => st.output.any fun
      | .send (.proposal p) => decide (p.viewNumber = v ∧ p.epoch = e)
      | .send (.revote r) => decide (r.view = v)
      | _ => false

/-- How a node stopped owing an obligation it did not act on in the step it owed it in. -/
inductive Ending where
  /-- It acted on it in this step: did what it owed, or held the action back. -/
  | acted (d : Nat)
  /-- It gave it up in this step, or no longer owed it after it, without having acted on it. -/
  | lapsed (d : Nat)
deriving DecidableEq, Repr

/--
How the node stopped owing `o`, from step `m` on, in a run `h`, by what `owed`
says: the first step after which it no longer owed `o`, or in which it held the
action back (`heldThrough`), if the run has one.
-/
def endingOf (owed : Array (List Obligation)) (h : History)
    (held : List HeldAt) (o : Obligation) (m : Nat) : Option Ending :=
  let owes := fun j o => (owed.getD j []).contains o
  ((List.range' m (h.length - m)).find? fun d => !owes (d + 1) o || heldThrough owes held o d).map
    fun d =>
      if (heldThrough owes held o d && !gaveUp held o d) || (h[d]?.map (does node · o)).getD false
      then .acted d
      else .lapsed d

/-- Whether storage confirming `stored` confirms the write `awaited`: the same write, or for a lock one at least as late. -/
def confirms (stored awaited : Write) : Bool :=
  match stored, awaited with
  | .lock v e, .lock v' e' => decide (e' < e ∨ (e' = e ∧ v' ≤ v))
  | a, b => a == b

/--
Each obligation the node owes after the last of `len` steps whose action it held
back, and never released though storage confirmed every write the action waited
for, with the step that held it back, by what `owed` says.

A held-back action is released by the confirmation of the last write it waits
for. A run can end before that, and the action is then still under way. One the
trace names no write for is not judged. An
obligation the node has given up since (`gaveUp`) had its action dropped.
-/
def unreleasedIn (owed : Array (List Obligation)) (len : Nat) (held : List HeldAt)
    (dropped : List DroppedAt) : List (Nat × Obligation) :=
  let owes := fun j o => (owed.getD j []).contains o
  ((owed.getD len []).filter fun o => !gaveUp held o (len - 1)).filterMap fun o =>
    let holds := held.filter fun x =>
      o.heldAs == some (x.kind, x.view, x.epoch) && x.step < len
        && (List.range' (x.step + 1) (len - x.step)).all fun j => owes j o
    match holds.foldl (init := (none : Option HeldAt)) fun acc x =>
        match acc with
        | some y => if y.line < x.line then some x else acc
        | none => some x with
    | some x =>
      if !x.waits.isEmpty && x.waits.all fun w => dropped.any fun d =>
          decide (x.line < d.line) && match d.kind with | .stored s => confirms s w | _ => false
      then some (x.step, o) else none
    | none => none

/--
Whether `g` answers the timer for `v` taken in its step `n`, or has grounds past
`v`.

The epoch the vote names is the one the node was in at the timer.
-/
def timerSettled (g : History) (n : Nat) (v : ViewNumber) : Bool :=
  let e := epochOfHistory cfg (g.take n)
  ((g.drop n).any fun st => (timeoutVotesOf st).any fun tv =>
      decide (tv.view = v ∧ tv.signer = node ∧ tv.data.epoch = e))
    || decide (v < viewOf cfg g)

/--
`NewProtocol.Prompt.timers`, read off the steps of a recorded run: every timer the
node takes, it answers in the same step, holds its timeout vote back for storage
there, in whatever epoch, or by then has grounds for a later view. A node outside
the committee of the epoch it is in at the timer (`member`) has no vote to cast.
-/
def TimersAnswered (member : EpochNumber → Bool) (h : History) (held : List HeldAt) : Prop :=
  ∀ n, n < h.length → ∀ v, History.TimerAt cfg (h.take (n + 1)) n v →
    History.TimerAnswered cfg node (h.take (n + 1)) n v
      ∨ (∃ u, History.ViewGround cfg (h.take (n + 1)) u ∧ v < u)
      ∨ (∃ x, x ∈ held ∧ x.kind = .timeout ∧ x.view = v ∧ x.step = n)
      ∨ member (epochOfHistory cfg (h.take n)) = false

/-- Whether `held` has a timeout vote for view `v` held back in step `n`. -/
def timeoutHeld (held : List HeldAt) (v : ViewNumber) (n : Nat) : Bool :=
  held.any fun x => x.kind == .timeout && x.view == v && x.step == n

/-- Each timer the node took and did not settle in its step, with the step. -/
def timerFaults (member : EpochNumber → Bool) (h : History) (held : List HeldAt) :
    List (Nat × ViewNumber) :=
  (List.range h.length).filterMap fun n =>
    match timerView cfg (h.take (n + 1)) n with
    | some v =>
      if timerSettled cfg node (h.take (n + 1)) n v || timeoutHeld held v n
          || !member (epochOfHistory cfg (h.take n))
      then none
      else some (n, v)
    | none => none

/-! ## Inputs the specification has no counterpart for -/

/--
How many blocks before an epoch's last are, with it, the epoch's transition, in
which the implementation votes and proposes only once it holds the next epoch's
DRB result (its `is_epoch_transition`).
-/
def transitionWindow : Nat := 3

/-- Whether block `b` is in the transition of its epoch, of `height` blocks. -/
def inTransition (b : BlockNumber) (height : Nat) : Bool :=
  b.toNat != 0 && height != 0
    && (decide (height - transitionWindow ≤ b.toNat % height) || b.toNat % height == 0)

/--
Whether an action of `kind` in epoch `e` on the block numbered `b` waits for the
DRB result of `drb`: a vote1 or a proposal after the first epoch, on a block in
its epoch's transition, for the epoch after. An action whose block is not known
is not read as waiting.
-/
def waitsForDrb (kind : HeldKind) (e : EpochNumber) (b : Option BlockNumber) (drb : EpochNumber) :
    Bool :=
  (kind == .vote1 || kind == .proposal) && decide (1 < e.toNat) && drb == e + 1
    && (b.map (inTransition · cfg.epochHeight)).getD false

/--
Whether acting on `o` waits for the DRB result the node asked for with `request`.

A vote1 names its block. A proposal owed does not, so the request has to be one
the node made for the block it built to propose in that view.
-/
def needsDrb (o : Obligation) (request : DrbRequest) : Bool :=
  match o with
  | .vote1 p => waitsForDrb cfg .vote1 p.epoch (some p.blockHeader.blockNumber) request.epoch
  | .propose e v => request.view == some v && waitsForDrb cfg .proposal e request.block request.epoch
  | _ => false

/--
The action an output is, if it is one a node can owe and hold back: what it is
held back as, its view and epoch, and the number of its block if it names one.
-/
def actionOf : Output → Option (HeldKind × ViewNumber × EpochNumber × Option BlockNumber)
  | .send (.vote1 v) =>
      if v.signer = node then some (.vote1, v.view, v.data.epoch, some v.data.blockNumber) else none
  | .send (.vote2 v) => if v.signer = node then some (.vote2, v.view, v.data.epoch, none) else none
  | .send (.proposal p) => some (.proposal, p.viewNumber, p.epoch, some p.blockHeader.blockNumber)
  | .send (.revote r) => some (.proposal, r.view, r.cert.data.epoch, none)
  | .send (.timeoutVote v) =>
      if v.signer = node then some (.timeout, v.view, v.data.epoch, none) else none
  | _ => none

/--
Each action the node owed already and took, or decided on and held back, only at
a dropped input that explains no wait, with the step the input joins. `owed line
kind v e` is whether the node owed the action by the lines before `line`.

The specification saw nothing between a step and a dropped input after it, so an
action the node owed after the step and acts on only at the dropped input waited
for an input the specification does not ask for. Three things explain such a
wait: the input confirms a write the action was held back for, it is the DRB
result the action waits for, or it says the block the proposal is of is built. An
action taken names its block. One held back does not, so it has to be one the
node asked that DRB result for (`requests`).

An action the node did not owe, it may take whenever it likes: a proposal in a
view it has left, say. A timeout vote answers a timer, which is a step.
-/
def droppedActionFaults (owed : Nat → HeldKind → ViewNumber → EpochNumber → Bool)
    (held : List HeldAt) (dropped : List DroppedAt) (requests : List (DrbRequest × Nat × Nat)) :
    List (Nat × HeldKind × ViewNumber × Dropped) :=
  dropped.flatMap fun d =>
    let taken := (d.output.filterMap (actionOf node)).filterMap fun (kind, v, e, b) =>
      let released := held.any fun x =>
        decide (x.line < d.line) && x.kind == kind && x.view == v && x.epoch == e
          && match d.kind with
            | .stored s => x.waits.any (confirms s)
            | _ => false
      let awaited : Bool := match d.kind with
        | .drbResult drb => waitsForDrb cfg kind e b drb
        | .blockBuilt u => kind == .proposal && u == v
        | _ => false
      if released || awaited || !owed d.line kind v e then none else some (d.step, kind, v, d.kind)
    let decided := (held.filter fun x => x.line == d.line && x.kind != .timeout).filterMap fun x =>
      let awaited : Bool := match d.kind with
        | .drbResult drb => requests.any fun (r, _, line) =>
            decide (line < d.line) && r.epoch == drb && r.view == some x.view
              && waitsForDrb cfg x.kind x.epoch r.block drb
        | .blockBuilt u => x.kind == .proposal && u == x.view
        | _ => false
      if awaited || !owed d.line x.kind x.view x.epoch then none
      else some (d.step, x.kind, x.view, d.kind)
    taken ++ decided

/--
Whether the DRB result `o` waits for, which the node had asked for, arrived at a
dropped input whose outputs join step `acted`.
-/
def drbArrivedInto (requests : List (DrbRequest × Nat × Nat)) (dropped : List DroppedAt)
    (o : Obligation) (acted : Nat) : Bool :=
  dropped.any fun d => d.step == acted && match d.kind with
    | .drbResult drb => requests.any fun (r, _, line) =>
        decide (line < d.line) && r.epoch == drb && needsDrb cfg o r
    | _ => false

/--
Whether the node had asked for the DRB result `o` waits for by step `last`, and
had not got it at a dropped input joining a step before `before`.
-/
def drbAwaited (requests : List (DrbRequest × Nat × Nat)) (dropped : List DroppedAt)
    (o : Obligation) (last before : Nat) : Bool :=
  requests.any fun (r, k, line) =>
    needsDrb cfg o r && k ≤ last
      && !dropped.any fun d => d.kind == .drbResult r.epoch && line < d.line && d.step < before

/--
Whether the state of the block `o` asks a vote1 for was found invalid at a
dropped input joining a step up to `last`. A node does not vote for such a
block. The specification cannot take a block back, and may come to owe the vote
only after the failure.
-/
def stateValidationFailedFor (dropped : List DroppedAt) (o : Obligation) (last : Nat) : Bool :=
  match o with
  | .vote1 p => dropped.any fun d =>
      d.kind == .stateValidationFailed p.viewNumber (blockHash p) && d.step ≤ last
  | _ => false

/-- Whether step `st` received the proposal of the block `c` certifies at its view, with a share. -/
def sharesBlock (c : Cert1) (st : Step) : Bool :=
  match st with
  | ⟨.proposal _ p (some _), _⟩ =>
      decide (p.viewNumber = c.view) && decide (blockHash p = c.data.blockHash)
  | _ => false

/--
Whether a vote2 obligation `o` the node acted on in step `m` (`endingOf`) waited
for the node's own share of its block: whether the share arrived in step `m` or a
step taken with it (`joined`).

An implementation may hold a vote2 back until it has its share of the block, and
stored it, so that a `Cert2` means a quorum stored shares of the payload and the
block survives every node restarting. The specification does not model restarts,
so it owes the vote2 without the share; a vote2 that waited for it is excused. A
vote2 held back for storage is acted on when it is held back, so one acted on
later than the share's arrival waited for something else.
-/
def shareArrived (h : History) (joined : List Nat) (m : Nat) : Obligation → Bool
  | .vote2 c => (List.range (m + 1)).any fun k =>
      decide (groupStart joined m m ≤ k) && (h[k]?.map (sharesBlock c)).getD false
  | _ => false

/--
Whether a vote2 obligation `o` is over a block at its own view whose share the
node had not received by step `last`.

Such a vote2 waits for the share (`shareArrived`), and an equivocating leader can
withhold the share for good, by sending the node a rival's. A re-vote's
certificate is at a later view than its block, and its vote2 waits for no share.

This accepts a divergence that lasts, not only a delay: the node never sends a
vote2 the specification owes. A leader that equivocates this way to some honest
nodes can keep its own view from a `Cert2`. The block keeps its `Cert1`, which
locks it and reaches later views through the timeout certificate, and it is
decided when a later block on it is. The specification promises no `Cert2` in a
faulty leader's view. The implementation would send the vote2 too if it computed
its own share from the payload it holds.
-/
def lacksShare (h : History) (last : Nat) : Obligation → Bool
  | .vote2 c =>
      let seen := h.take (last + 1)
      (proposalsHeld cfg seen).any (fun p =>
          decide (p.viewNumber = c.view) && decide (blockHash p = c.data.blockHash))
        && !seen.any (sharesBlock c)
  | _ => false

/-! ## Soundness -/

variable {cfg leader node}

theorem mem_owedAfter {h : History} {j : Nat} (hj : j ≤ h.length) {o : Obligation} :
    o ∈ (owedAfter cfg leader node h).getD j [] ↔ Owed cfg leader node (h.take j) o := by
  have hj' : j < h.length + 1 := Nat.lt_succ_of_le hj
  simp only [owedAfter, Array.getD_eq_getD_getElem?, Array.getElem?_ofFn, hj', dite_true,
    Option.getD_some, List.mem_filter, decide_eq_true_eq]
  exact ⟨And.right, fun ho => ⟨NewProtocolImpl.mem_candidates rfl ho, ho⟩⟩

/--
**A trace with no step fault obeys `StepPrompt`**: whatever its node owed after a
step, it acted on in that step.
-/
theorem checkSteps_sound {h : History} {joined : List Nat}
    {held : List HeldAt}
    (hp : stepFaults cfg leader node h joined held = []) :
    StepPrompt cfg leader node h joined held := by
  intro m hm hj o ho
  have hnone := List.flatMap_eq_nil_iff.mp hp m (List.mem_range.mpr hm)
  have hc : joined.contains m = false := by simpa using hj
  simp only [hc, Bool.false_eq_true, ite_false, List.map_eq_nil_iff, List.filter_eq_nil_iff,
    Bool.not_eq_true', Bool.not_eq_false] at hnone
  have hheld := hnone o ((mem_owedAfter (by omega)).mpr ho)
  simp only [heldThrough, List.any_eq_true, Bool.and_eq_true, Bool.or_eq_true, List.all_eq_true,
    List.mem_range'_1, decide_eq_true_eq, beq_iff_eq, List.contains_iff_mem,
    Option.any_eq_true] at hheld
  obtain ⟨x, hmem, hk, hcase⟩ := hheld
  refine ⟨x, hmem, hk, ?_⟩
  rcases hcase with ⟨hw, hall⟩ | ⟨hw, hv⟩
  · exact Or.inl ⟨hw, fun j hj1 hj2 => (mem_owedAfter (by omega)).mp (hall j ⟨hj1, by omega⟩)⟩
  · exact Or.inr ⟨hw, hv⟩

theorem timerSettled_of {g : History} {n : Nat} {v : ViewNumber}
    (hs : timerSettled cfg node g n v = true) :
    g.TimerAnswered cfg node n v ∨ ∃ u, g.ViewGround cfg u ∧ v < u := by
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
  · exact Or.inr ⟨_, (inView_viewOf cfg g).1, hlt⟩

/--
**A trace with no timer fault obeys `TimersAnswered`**: every timer its node took,
it answered in that step, held the answer back for storage, or had grounds past
its view by then.
-/
theorem checkTimers_sound {member : EpochNumber → Bool} {h : History} {held : List HeldAt}
    (hp : timerFaults cfg node member h held = []) : TimersAnswered cfg node member h held := by
  intro n hn v ht
  have hnone := List.filterMap_eq_nil_iff.mp hp n (List.mem_range.mpr hn)
  simp only [timerView_eq_some.mpr ht] at hnone
  cases hs : timerSettled cfg node (h.take (n + 1)) n v
  · cases hc : timeoutHeld held v n
    · cases hm : member (epochOfHistory cfg (h.take n))
      · exact Or.inr (Or.inr (Or.inr rfl))
      · simp only [hs, hc, hm, Bool.or_false, Bool.not_true, Bool.false_eq_true, ite_false,
          reduceCtorEq] at hnone
    · simp only [timeoutHeld, List.any_eq_true, Bool.and_eq_true, beq_iff_eq] at hc
      obtain ⟨x, hmem, ⟨hw, hu⟩, hk⟩ := hc
      exact Or.inr (Or.inr (Or.inl ⟨x, hmem, hw, hu, hk⟩))
  · rcases timerSettled_of hs with ha | hg
    · exact Or.inl ha
    · exact Or.inr (Or.inl hg)

end NewProtocolDiff
