module

public import NewProtocolImpl.Decide
public import NewProtocolSpec.Network

/-!
# The machine

A node whose state is its history. Each step answers a timer or one-honest
indication when the node is in a view that calls for it, then takes every
obligation the history holds, in one pass over the candidates the inputs name:
certificates for decides and vote2s, proposals and re-vote requests for vote1s,
and the node's view for a proposal or re-vote request.

It is slow, rescanning the history at every step, and is not meant to be
anything else: it shows the specification can be met, by an eager node that
leaves nothing owed after any step (`NewProtocolImpl.Conformance`).

Some choices here bind nobody. A decide walks back from the committed block
through held parents, stopping at genesis, at a view already decided or at a
parent it does not hold. A `Cert2` after a re-vote may commit more than one held
block, if their hashes agree, and the node decides each. A leader proposes when it
can and asks for a re-vote only when it cannot. A proposal it builds has identity
zero: the network assigns identities, and no rule reads one.
-/

@[expose] public section

namespace NewProtocolImpl

open NewProtocol NewProtocol.Lists History

variable (cfg : Config) (leader : EpochNumber → ViewNumber → Option PubKey) (node : PubKey)

/--
The timeout vote a timer or one-honest indication calls for, if the node's view
calls for one. It names the node's epoch and its lock.
-/
def timeoutAnswer (h : History) : Input → List Output
  | .timeout v =>
    if viewOf cfg h = v then [.send (.timeoutVote ⟨⟨epochOfHistory cfg h, lockOf cfg h⟩, v, node⟩)]
    else []
  | .timeoutOneHonest v =>
    if viewOf cfg h ≤ v then [.send (.timeoutVote ⟨⟨epochOfHistory cfg h, lockOf cfg h⟩, v, node⟩)]
    else []
  | _ => []

/-- The epochs the node could propose or ask for a re-vote in, at its view. -/
def proposeEpochs (h : History) : List EpochNumber :=
  ((proposalCandidates cfg h (viewOf cfg h)).map (·.epoch)
    ++ (revoteCandidates cfg h (viewOf cfg h)).map (·.cert.data.epoch)).eraseDups

/-- Every obligation the history's inputs could give rise to. -/
def candidates (h : History) : List Obligation :=
  (cert2sHeld h).map .decide ++ (cert1sHeld cfg h).map .vote2
    ++ (receivedProposals h).map (fun x => .vote1 x.2.1)
    ++ (receivedRevotes h).map (fun x => .vote1Again x.2)
    ++ (proposeEpochs cfg h).map (.propose · (viewOf cfg h))

/--
The parent of `b` a decide may continue to: held, after genesis and before `b`, at
a view not decided yet and not in `seen`, the views this pass has delivered.
-/
def nextInChain (h : History) (seen : List ViewNumber) (b : Block) : Option Block :=
  (proposalsHeld cfg h).find? fun q =>
    decide (q.viewNumber ≤ b.parentCert.view ∧ blockHash q = b.parentCert.data.blockHash
      ∧ cfg.anchorView < q.viewNumber ∧ q.viewNumber ∉ decidedViews h
      ∧ q.viewNumber < b.viewNumber ∧ q.viewNumber ∉ seen)

/-- The blocks a decide of `b` delivers, newest first. -/
def chainFrom (h : History) (seen : List ViewNumber) : Nat → Block → List Block
  | 0, b => [b]
  | fuel + 1, b =>
    match nextInChain cfg h seen b with
    | some q => b :: chainFrom h seen fuel q
    | none => [b]

/--
The decides a `Cert2` calls for: one for each held block it commits, in turn,
skipping a block at a view decided already or delivered by an earlier one of them.

So no view is delivered twice, even when a block was received more than once.
-/
def decidesFor (h : History) (c : Cert2) (c1 : Block → Cert1) : List ViewNumber → List Block → List Output
  | _, [] => []
  | seen, b :: bs =>
    if b.viewNumber ∈ seen ∨ b.viewNumber ∈ decidedViews h then decidesFor h c c1 seen bs
    else
      .decided (chainFrom cfg h seen b.viewNumber.toNat b) (c1 b) c
        :: decidesFor h c c1 (seen ++ (chainFrom cfg h seen b.viewNumber.toNat b).map (·.viewNumber)) bs

/-- A `Cert1` the node holds over `b`, to deliver beside a decide of it. -/
def cert1For (h : History) (b : Block) : Option Cert1 :=
  (cert1sHeld cfg h).find? fun c1 => decide (Certifies c1 b)

/-- The outputs that discharge an obligation. -/
def act (h : History) : Obligation → List Output
  | .vote1 p => [.send (.vote1 ⟨⟨blockHash p, p.epoch, p.blockHeader.blockNumber⟩, p.viewNumber, node⟩)]
  | .vote1Again r => [.send (.vote1 ⟨r.cert.data, r.view, node⟩)]
  | .vote2 c => [.send (.vote2 ⟨c.data.toVote2, c.view, node⟩)]
  | .decide c =>
    decidesFor cfg h c (fun b => (cert1For cfg h b).getD cfg.anchorCert) []
      ((proposalsHeld cfg h).filter fun b =>
        decide (Commits c b ∧ cfg.anchorView < b.viewNumber) && (cert1For cfg h b).isSome)
  | .propose e v =>
    match (proposalCandidates cfg h v).find?
        (fun p => decide (p.epoch = e ∧ ProposalJustifiedB cfg leader node h p)) with
    | some p => [.send (.proposal p)]
    | none =>
      match (revoteCandidates cfg h v).find?
          (fun r => decide (r.cert.data.epoch = e ∧ RevoteJustifiedB cfg leader node h r)) with
      | some r => [.send (.revote r)]
      | none => []

/--
Whether the obligation's view is still open: the node's view for a vote1 or a
proposal, not timed out, above the floor for a decide, and not yet acted in, in
the obligation's epoch.

A cheap test tried before `Owed`: most candidates at any step are in views long
closed, and this rules them out with a few scans (`fresh_of_owed`).
-/
def fresh (h : History) : Obligation → Bool
  | .vote1 p => decide (viewOf cfg h = p.viewNumber) && !decide (TimedOutB h p.viewNumber)
      && (vote1sSent h).all (fun v => v.data.epoch != p.epoch || v.view != p.viewNumber)
  | .vote1Again r => decide (viewOf cfg h = r.view) && !decide (TimedOutB h r.view)
      && (vote1sSent h).all (fun v => v.data.epoch != r.cert.data.epoch || v.view != r.view)
  | .vote2 c => !decide (TimedOutB h c.view)
      && (vote2sSent h).all (fun v => v.data.epoch != c.data.epoch || v.view != c.view)
  | .decide c => decide (AboveFloorB cfg h c.view)
  | .propose e v => decide (viewOf cfg h = v) && !decide (TimedOutB h v)
      && (proposalsSent h).all (fun p => p.viewNumber != v || p.epoch != e)
      && (revotesSent h).all (fun r => r.view != v || r.cert.data.epoch != e)

/-- Take each obligation in turn that is still owed, given what the step has output so far. -/
def discharge (pre : History) (i : Input) : List Obligation → List Output → List Output
  | [], out => out
  | o :: os, out =>
    if fresh cfg (pre ++ [Step.mk i out]) o = true ∧ Owed cfg leader node (pre ++ [Step.mk i out]) o then
      discharge pre i os (out ++ act cfg leader node (pre ++ [Step.mk i out]) o)
    else discharge pre i os out

/-- The step the machine takes on input `i` after history `pre`. -/
def step (pre : History) (i : Input) : Step :=
  ⟨i, discharge cfg leader node pre i (candidates cfg (pre ++ [Step.mk i []]))
    (timeoutAnswer cfg node pre i)⟩

/-- The history after the machine takes the first `n` inputs. -/
def historyOf (inputs : Nat → Input) : Nat → History
  | 0 => []
  | n + 1 => historyOf inputs n ++ [step cfg leader node (historyOf inputs n) (inputs n)]

/-- The machine's trace on an input sequence. -/
def traceOf (inputs : Nat → Input) : Trace :=
  fun n => step cfg leader node (historyOf cfg leader node inputs n) (inputs n)

theorem traceOf_history (inputs : Nat → Input) (n : Nat) :
    (traceOf cfg leader node inputs).history n = historyOf cfg leader node inputs n := by
  induction n with
  | zero => rfl
  | succ n ih =>
    simp only [Trace.history, List.range_succ, List.map_append, List.map_cons, List.map_nil] at ih ⊢
    rw [ih]; rfl

end NewProtocolImpl
