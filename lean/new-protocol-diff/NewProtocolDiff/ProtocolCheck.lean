module

public import NewProtocolDiff.Check
public import NewProtocolImpl.Steps

/-!
# Checking a trace against the rest of the protocol

The rules `NewProtocol.ProtocolHistory` adds to the signing rules, decided on a
recorded trace: when a node may time out, and what it may propose. That it answers
its timers in time is checked with the obligations (`NewProtocolDiff.PromptCheck`). Unlike the signing rules, these read the node's view, epoch and lock,
which the specification derives from the history as the latest each has grounds
for. Here each ground is collected into a list, and the latest is the largest
element.

`checkProtocol_sound` is what makes a pass mean the trace obeys
`ProtocolHistory`, for the leader schedule the trace names.
-/

@[expose] public section

namespace NewProtocolDiff

open NewProtocol NewProtocol.Lists NewProtocolImpl History

variable (cfg : Config) (h : History)

/-! ## The rules, step by step -/

variable (node : PubKey) (leader : EpochNumber → ViewNumber → Option PubKey)

/-- Whether step `m` took a timer or one-honest indication for view `v` (`History.TimerAt`). -/
def timerAtB (m : Nat) (v : ViewNumber) : Bool :=
  match h[m]? with
  | some st =>
    match st.input with
    | .timeout v' => decide (v' = v ∧ viewOf cfg (h.take m) = v)
    | .timeoutOneHonest v' => decide (v' = v ∧ viewOf cfg (h.take m) ≤ v)
    | _ => false
  | none => false

/--
Whether a timeout vote sent in step `n` is justified (`ProtocolHistory.timeoutJustified`).

The timer is searched for from step `n` back, since it is almost always the step
that sends the vote.
-/
def timeoutJustifiedBy (n : Nat) (vote : TimeoutVote) : Bool :=
  decide (vote.signer = node)
    && (List.range (n + 1)).reverse.any fun m =>
      timerAtB cfg h m vote.view && decide (epochOfHistory cfg (h.take m) = vote.data.epoch)
        && decide (vote.data.lock ∈ cert1sHeld cfg (h.take m))

/-- Why a proposal is not one the node may make, given that `ProposalJustifiedB` failed. -/
def proposalReason (post : History) (p : Proposal) : String :=
  if !decide (leader p.epoch p.viewNumber = some node) then "not the leader"
  else if !decide (ProposalWellFormed cfg p) then "not well formed"
  else if !decide (CertJustifiedB cfg post p.parentCert p.timeoutEvidence) then
    match p.timeoutEvidence with
    | none => "parent certificate not held"
    | some _ => s!"after a timeout, parent was never the lock since the timeout certificate \
        (lock view {(lockView cfg post).toNat}, parent view {p.parentCert.view.toNat})"
  else if !decide (∃ parent ∈ proposalsHeld cfg post, parent.viewNumber ≤ p.parentCert.view
      ∧ p.parentCert.data.blockHash = blockHash parent) then
    "parent not held"
  else if !decide (OpensB cfg post p) then
    "opens an epoch without naming its parent at the parent's view, behind a Cert2 over it"
  else if !decide (SafeParentB p) then
    "parent before the timeout certificate's lock, or the certificate is of another epoch"
  else if !decide (NotBehindB cfg post p.epoch) then "for an epoch the node has left"
  else if !decide (p.viewNumber ≤ viewOf cfg post) then "for a view the node has not reached"
  else "header not built for this view and parent"

/-- Whether a proposal is one the node may make (`ProposalJustified`), and if not, why. -/
def proposalFault (post : History) (p : Proposal) : Option String :=
  if decide (ProposalJustifiedB cfg leader node post p) then none
  else some (proposalReason cfg node leader post p)

/-- Why a re-vote request is not one the node may send, given that `RevoteJustifiedB` failed. -/
def revoteReason (post : History) (r : RevoteRequest) : String :=
  if !decide (leader r.cert.data.epoch r.view = some node) then "not the leader"
  else if !decide (RevoteWellFormed cfg r) then "not well formed"
  else if !decide (CertJustifiedB cfg post r.cert r.timeoutEvidence) then
    "certificate not one the node may vote on again"
  else if !decide (SafeRevoteB r) then
    "certificate before the timeout certificate's lock, or the certificate is of another epoch"
  else if !decide (NotBehindB cfg post r.cert.data.epoch) then "for an epoch the node has left"
  else "for a view the node has not reached"

/-- Whether a re-vote request is one the node may send (`RevoteJustified`), and if not, why. -/
def revoteFault (post : History) (r : RevoteRequest) : Option String :=
  if decide (RevoteJustifiedB cfg leader node post r) then none
  else some (revoteReason cfg node leader post r)

/-- Whether a vote1 was on a proposal or re-vote request from its view's leader (`ProtocolHistory.vote1Leader`). -/
def vote1LeaderBy (post : History) (v : Vote1) : Bool :=
  decide (v.view ≤ viewOf cfg post)
    && ((receivedProposals post).any (fun x => decide (leader x.2.1.epoch x.2.1.viewNumber = some x.1
        ∧ Vote1For v x.2.1 ∧ NotBehindB cfg post x.2.1.epoch))
      || (receivedRevotes post).any fun x => decide (leader x.2.cert.data.epoch x.2.view = some x.1
        ∧ Vote1Again v x.2 ∧ NotBehindB cfg post x.2.cert.data.epoch))

/-- Why a timeout vote sent in step `n` is not justified, given that `timeoutJustifiedBy` failed. -/
def timeoutReason (n : Nat) (st : Step) : String :=
  match (timeoutVotesOf st).find? (!timeoutJustifiedBy cfg h node n ·) with
  | some tv => s!"timeoutJustified: a vote for view {tv.view.toNat} naming epoch \
      {tv.data.epoch.toNat}, with no timer for that view at which the node was in that epoch \
      and held the lock"
  | none => "timeoutJustified"

/-- The first rule step `n` breaks, and why. -/
def stepFault (n : Nat) (st : Step) : Option String :=
  if !(vote1sOf st).all (vote1LeaderBy cfg leader (h.take (n + 1))) then
    some s!"vote1Leader: a vote1 not on a proposal or re-vote request its view's leader sent, \
      or for a view not reached or an epoch left"
  else if !(timeoutVotesOf st).all (timeoutJustifiedBy cfg h node n) then
    some (timeoutReason cfg h node n st)
  else
    match (proposalsOf st).findSome? fun p =>
        (proposalFault cfg node leader (h.take (n + 1)) p).map (s!"proposeJustified: {·}") with
    | some f => some f
    | none => (revotesOf st).findSome? fun r =>
        (revoteFault cfg node leader (h.take (n + 1)) r).map (s!"revoteJustified: {·}")

/-- `ProtocolHistory.proposeOnce`, for proposals. -/
def CheckProposeOnce : Prop :=
  ∀ p ∈ proposalsSent h, ∀ p' ∈ proposalsSent h, p.epoch = p'.epoch → p.viewNumber = p'.viewNumber → p = p'

/-- `ProtocolHistory.proposeOnce`, for re-vote requests. -/
def CheckRevoteOnce : Prop :=
  ∀ r ∈ revotesSent h, (∀ r' ∈ revotesSent h, r'.cert.data.epoch = r.cert.data.epoch → r'.view = r.view → r' = r)
    ∧ ∀ p ∈ proposalsSent h, p.epoch = r.cert.data.epoch → p.viewNumber ≠ r.view

instance : Decidable (CheckProposeOnce h) := by unfold CheckProposeOnce; infer_instance

instance : Decidable (CheckRevoteOnce h) := by unfold CheckRevoteOnce; infer_instance

/-- The first step breaking a protocol rule, with the rule and why. -/
def protocolFault : Option (Nat × String) :=
  match (List.range h.length).findSome? fun n =>
      (h[n]?.bind (stepFault cfg h node leader n)).map (n, ·) with
  | some fault => some fault
  | none =>
    if !decide (CheckProposeOnce h) then some (h.length, "proposeOnce")
    else if !decide (CheckRevoteOnce h) then some (h.length, "proposeOnce: re-vote requests")
    else none

variable {cfg h}

/-! ## Soundness -/

variable {node leader}

theorem timerAt_of {m : Nat} {v : ViewNumber} (ht : timerAtB cfg h m v = true) : h.TimerAt cfg m v := by
  unfold timerAtB at ht
  split at ht
  · rename_i st hst
    refine ⟨st, hst, ?_⟩
    revert ht
    cases st.input <;> simp only [decide_eq_true_eq, Bool.false_eq_true, false_implies]
    · rintro ⟨rfl, hv⟩
      exact Or.inl ⟨rfl, hv ▸ inView_viewOf cfg _⟩
    · rintro ⟨rfl, hv⟩
      exact Or.inr ⟨rfl, _, inView_viewOf cfg _, hv⟩
  · cases ht

theorem timeoutJustified_of {n : Nat} {vote : TimeoutVote} (hj : timeoutJustifiedBy cfg h node n vote = true) :
    vote.signer = node ∧ ∃ m ≤ n, h.TimerAt cfg m vote.view ∧ (h.upTo m).InEpoch cfg vote.data.epoch
      ∧ (h.upTo m).HasCert1 cfg vote.data.lock := by
  simp only [timeoutJustifiedBy, Bool.and_eq_true, decide_eq_true_eq, List.any_eq_true,
    List.mem_reverse, List.mem_range] at hj
  obtain ⟨hsig, m, hm, ⟨ht, hep⟩, hlock⟩ := hj
  exact ⟨hsig, m, Nat.lt_succ_iff.mp hm, timerAt_of ht, hep ▸ inEpoch_epochOfHistory cfg _,
    hasCert1_of_mem hlock⟩

theorem proposalJustified_of {post : History} {p : Proposal}
    (hf : proposalFault cfg node leader post p = none) :
    ProposalJustified cfg leader node post p := by
  unfold proposalFault at hf
  by_cases hj : ProposalJustifiedB cfg leader node post p
  · exact proposalJustifiedB_iff.mp hj
  · simp [hj] at hf

theorem revoteJustified_of {post : History} {r : RevoteRequest}
    (hf : revoteFault cfg node leader post r = none) :
    RevoteJustified cfg leader node post r := by
  unfold revoteFault at hf
  by_cases hj : RevoteJustifiedB cfg leader node post r
  · exact revoteJustifiedB_iff.mp hj
  · simp [hj] at hf

theorem vote1Leader_of {post : History} {v : Vote1} (hv : vote1LeaderBy cfg leader post v = true) :
    (∃ u, post.ViewGround cfg u ∧ v.view ≤ u)
      ∧ ((∃ sender p vid, post.Received (.proposal sender p (some vid))
          ∧ leader p.epoch p.viewNumber = some sender ∧ Vote1For v p ∧ NotBehind cfg post p.epoch)
        ∨ ∃ sender r, post.Received (.revote sender r)
          ∧ leader r.cert.data.epoch r.view = some sender ∧ Vote1Again v r
          ∧ NotBehind cfg post r.cert.data.epoch) := by
  simp only [vote1LeaderBy, Bool.and_eq_true, Bool.or_eq_true, List.any_eq_true,
    decide_eq_true_eq] at hv
  obtain ⟨hreach, ⟨⟨s, q, vid⟩, hx, hl, hfor, hnb⟩ | ⟨⟨s, r⟩, hx, hl, hag, hnb⟩⟩ := hv
  · exact ⟨reached_iff_viewOf.mpr hreach,
      Or.inl ⟨s, q, vid, mem_receivedProposals.mp hx, hl, hfor, notBehindB_iff.mp hnb⟩⟩
  · exact ⟨reached_iff_viewOf.mpr hreach,
      Or.inr ⟨s, r, mem_receivedRevotes.mp hx, hl, hag, notBehindB_iff.mp hnb⟩⟩

theorem stepFault_none {n : Nat} {st : Step} (hf : stepFault cfg h node leader n st = none) :
    (∀ vote ∈ vote1sOf st, vote1LeaderBy cfg leader (h.take (n + 1)) vote = true)
      ∧ (∀ vote ∈ timeoutVotesOf st, timeoutJustifiedBy cfg h node n vote = true)
      ∧ (∀ p ∈ proposalsOf st, proposalFault cfg node leader (h.take (n + 1)) p = none)
      ∧ ∀ r ∈ revotesOf st, revoteFault cfg node leader (h.take (n + 1)) r = none := by
  unfold stepFault at hf
  by_cases ha : (vote1sOf st).all (vote1LeaderBy cfg leader (h.take (n + 1))) = true <;>
    by_cases h1 : (timeoutVotesOf st).all (timeoutJustifiedBy cfg h node n) = true <;>
    simp only [ha, h1, Bool.not_true, Bool.not_false, Bool.false_eq_true, ite_false, ite_true,
      reduceCtorEq] at hf
  refine ⟨List.all_eq_true.mp ha, List.all_eq_true.mp h1, ?_⟩
  split at hf
  · cases hf
  · rename_i hp
    rw [List.findSome?_eq_none_iff] at hp hf
    exact ⟨fun p hm => by simpa using hp p hm, fun r hm => by simpa using hf r hm⟩

/--
**A trace that passes both checks obeys the protocol rules**, for the leader
schedule the check was given, and given that the validity reports it received
were truthful.
-/
theorem checkProtocol_sound (hs : checkSafe cfg node h = true) (hvalid : ValidityTruthful h)
    (hp : protocolFault cfg h node leader = none) : ProtocolHistory cfg leader node (fun _ => True) h := by
  unfold protocolFault at hp
  split at hp
  · cases hp
  rename_i hsteps
  have honce : CheckProposeOnce h := by
    by_cases hno : CheckProposeOnce h
    · exact hno
    · simp [hno] at hp
  have hronce : CheckRevoteOnce h := by
    by_cases hno : CheckRevoteOnce h
    · exact hno
    · simp [hno, honce] at hp
  rw [List.findSome?_eq_none_iff] at hsteps
  have hstep : ∀ n st, h[n]? = some st → stepFault cfg h node leader n st = none := by
    intro n st hst
    obtain ⟨hn, _⟩ := List.getElem?_eq_some_iff.mp hst
    have := hsteps n (List.mem_range.mpr hn)
    simp only [hst, Option.bind_some, Option.map_eq_none_iff] at this
    exact this
  have hsafe := checkSafe_sound hs hvalid (fun _ => True)
  refine ⟨hsafe, ?_, ?_, ?_, ?_, fun m m' v e hs hs' hv hv' he he' _ => ?_⟩
  · rintro n vote ⟨st, hst, hmem⟩ _
    exact vote1Leader_of ((stepFault_none (hstep n st hst)).1 vote (mem_vote1sOf.mpr hmem))
  · intro n st vote hst hmem _
    exact timeoutJustified_of
      ((stepFault_none (hstep n st hst)).2.1 vote (mem_timeoutVotesOf hmem))
  · rintro n p ⟨st, hst, hmem⟩ _
    exact proposalJustified_of
      ((stepFault_none (hstep n st hst)).2.2.1 p (mem_proposalsOf hmem))
  · rintro n r ⟨st, hst, hmem⟩ _
    exact revoteJustified_of
      ((stepFault_none (hstep n st hst)).2.2.2 r (mem_revotesOf_iff.mpr hmem))
  · refine NewProtocolImpl.global_once ⟨fun v v' a b => hsafe.vote1Once v v' a b trivial,
      fun v v' a b => hsafe.vote2Once v v' a b trivial, ?_, ?_⟩ m m' v e hs hs' hv hv' he he'
    · rintro p p' ⟨st, hst, hm⟩ ⟨st', hst', hm'⟩ he hv
      exact honce p (List.mem_flatMap.mpr ⟨st, hst, mem_proposalsOf hm⟩)
        p' (List.mem_flatMap.mpr ⟨st', hst', mem_proposalsOf hm'⟩) he hv
    · intro r hr
      obtain ⟨h1, h2⟩ := hronce r (mem_revotesSent.mpr hr)
      exact ⟨fun r' hr' he hv => h1 r' (mem_revotesSent.mpr hr') he hv,
        fun p hp he => h2 p (mem_proposalsSent.mpr hp) he⟩

end NewProtocolDiff
