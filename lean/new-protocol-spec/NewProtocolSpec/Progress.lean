module

public import NewProtocolSpec.Progress.Defs
public import NewProtocolSpec.Progress.Lemmas
public import NewProtocolSpec.Safety

/-!
# Conditional progress

What an owed action is worth. The four results at the head of the file take an
action owed inside a window (`Vote1Owed` and its three companions) and conclude
that the action is *taken*: the step that consumes `WeaklyFair`, and the only
place in the specification where an obligation turns into an output.

None of this is liveness. Nothing here says a window ever opens — that needs the
delivery and timing assumptions the specification does not have — so every
result is conditional on the environment having done its part. What they do is
establish that the rules leave no way to stall once it has: inside a window an
enabledness predicate cannot lapse, and the mark obligations leave no way to
retire an action in silence. What a window rules out is the node overtaking the
view itself, which is not stalling.

The rest cross from one node to the network. `cert1_forms` and `cert2_forms` turn
a quorum's worth of votes into a certificate, and into the *same* certificate the
safety results speak about, which is what `LiveNetwork.netRun` is for.
`cert1_forms_of_owed` and `cert2_forms_of_owed` say it from the obligations
instead, which is the form a delivery argument reaches first, and are where
`LiveNetwork.fair` is used. `quorum_on_chain` is the two halves in one statement:
what progress produces, no-fork governs.
-/

@[expose] public section

namespace NewProtocol

/-! ## One node takes the action it owes -/

/--
**An owed vote1 is cast**, for the block the node admitted.

The argument is one step of fairness and two of bookkeeping. Suppose the vote is
never cast at or after the window opens. Then nothing sets the freshness mark
(`StepSpec.vote1Marked`, `GcSpec.voted1Sound`) and nothing else in `Vote1Enabled`
can lapse inside the window, so the vote is owed at every point from there on —
which is exactly `WeaklyFair`'s antecedent. Its conclusion is an emission at or
after the same step (`Run.EmitsFrom`), which contradicts the supposition; an
older vote would not, which is why the anchor is worth having.

The block is read off the step that cast the vote, and the *first* such step is
the one taken: past it the vote is on the run's record, and that is exactly where
a window stops saying anything. `SafetySpec.vote1Justified` says the vote
signs whatever the node has admitted at that view, and
`StepSpec.contentRetained` — which is not keyed on the bar — says that is still
`p` when the step ends.
-/
theorem vote1_cast {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {r : Run cfg (StepSpec cfg leader node)} {p : Proposal}
    (hfair : WeaklyFair r) (howed : Vote1Owed r p) :
    ∃ j, ∃ vote : Vote1, Output.send (.vote1 vote) ∈ (Run.event r j).outputs
      ∧ vote.view = p.viewNumber ∧ vote.data.blockHash = blockHash p ∧ vote.data.epoch = p.epoch ∧ vote.signer = node := by
  obtain ⟨n, hen, hw⟩ := howed
  have hsome : ∃ j, n ≤ j ∧ ∃ vote : Vote1,
      Output.send (.vote1 vote) ∈ (Run.event r j).outputs ∧ vote.view = p.viewNumber := by
    by_cases hex : ∃ j, n ≤ j ∧ ∃ vote : Vote1,
        Output.send (.vote1 vote) ∈ (Run.event r j).outputs ∧ vote.view = p.viewNumber
    · exact hex
    · exfalso
      have hno : ∀ i, n ≤ i → ∀ vote : Vote1,
          Output.send (.vote1 vote) ∈ (Run.event r i).outputs → vote.view ≠ p.viewNumber :=
        fun i hi vote hmem hview => hex ⟨i, hi, vote, hmem, hview⟩
      obtain ⟨j, hj, o, hmem, vote, rfl, hview⟩ :=
        hfair.vote1 p n fun m hm =>
          vote1Enabled_upTo hw (fun i hi _ => hno i hi) hen m hm (Nat.le_refl m)
      exact hno j hj vote hmem hview
  obtain ⟨j₀, hj₀⟩ := hsome
  obtain ⟨j, ⟨hj, vote, hmem, hview⟩, hmin⟩ :=
    exists_least (P := fun j => n ≤ j ∧ ∃ vote : Vote1,
      Output.send (.vote1 vote) ∈ (Run.event r j).outputs ∧ vote.view = p.viewNumber)
      j₀ j₀ (Nat.le_refl j₀) hj₀
  have hpend : Vote1Pending r p n j :=
    fun i hi hij vote hmemi hviewi => hmin i hij ⟨hi, vote, hmemi, hviewi⟩
  have henj : Vote1Enabled (Run.state r j) p :=
    vote1Enabled_upTo hw hpend hen j hj (Nat.le_refl j)
  obtain ⟨input, output, -, hs, hin⟩ := emit_step r hmem
  obtain ⟨q, hjq, hviewq, hsigner, hdata, hepq⟩ := SafetySpec.vote1Justified hs.toSafetySpec vote hin
  have hadm : (Run.state r (j + 1)).admitted p.viewNumber = some p :=
    (retainsVote_of_step hs (hw.floor j hj hpend)).admitted p henj.1.proposalAdmitted
  have hqp : q = p := by
    have h1 := hjq.proposalAdmitted
    rw [← hviewq, hview] at h1
    exact Option.some_inj.mp (h1.symm.trans hadm)
  exact ⟨j, vote, hmem, hview, hqp ▸ hdata, hqp ▸ hepq, hsigner⟩

/--
**An owed vote2 is cast**, for the block the node admitted.

As `vote1_cast`, with `SafetySpec.vote2Justified` reading the block.
-/
theorem vote2_cast {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {r : Run cfg (StepSpec cfg leader node)} {p : Proposal}
    (hfair : WeaklyFair r) (howed : Vote2Owed r p) :
    ∃ j, ∃ vote : Vote2, Output.send (.vote2 vote) ∈ (Run.event r j).outputs
      ∧ vote.view = p.viewNumber ∧ vote.data.blockHash = blockHash p ∧ vote.data.epoch = p.epoch ∧ vote.signer = node := by
  obtain ⟨n, hen, hw⟩ := howed
  have hsome : ∃ j, n ≤ j ∧ ∃ vote : Vote2,
      Output.send (.vote2 vote) ∈ (Run.event r j).outputs ∧ vote.view = p.viewNumber := by
    by_cases hex : ∃ j, n ≤ j ∧ ∃ vote : Vote2,
        Output.send (.vote2 vote) ∈ (Run.event r j).outputs ∧ vote.view = p.viewNumber
    · exact hex
    · exfalso
      have hno : ∀ i, n ≤ i → ∀ vote : Vote2,
          Output.send (.vote2 vote) ∈ (Run.event r i).outputs → vote.view ≠ p.viewNumber :=
        fun i hi vote hmem hview => hex ⟨i, hi, vote, hmem, hview⟩
      obtain ⟨j, hj, o, hmem, vote, rfl, hview⟩ :=
        hfair.vote2 p n fun m hm =>
          vote2Enabled_upTo hw (fun i hi _ => hno i hi) hen m hm (Nat.le_refl m)
      exact hno j hj vote hmem hview
  obtain ⟨j₀, hj₀⟩ := hsome
  obtain ⟨j, ⟨hj, vote, hmem, hview⟩, hmin⟩ :=
    exists_least (P := fun j => n ≤ j ∧ ∃ vote : Vote2,
      Output.send (.vote2 vote) ∈ (Run.event r j).outputs ∧ vote.view = p.viewNumber)
      j₀ j₀ (Nat.le_refl j₀) hj₀
  have hpend : Vote2Pending r p n j :=
    fun i hi hij vote hmemi hviewi => hmin i hij ⟨hi, vote, hmemi, hviewi⟩
  have henj : Vote2Enabled cfg (Run.state r j) p :=
    vote2Enabled_upTo hw hpend hen j hj (Nat.le_refl j)
  obtain ⟨input, output, -, hs, hin⟩ := emit_step r hmem
  obtain ⟨q, hjq, hviewq, hsigner, hdata, hepq⟩ := SafetySpec.vote2Justified hs.toSafetySpec vote hin
  have hadm : (Run.state r (j + 1)).admitted p.viewNumber = some p :=
    (retainsVote_of_step hs (hw.floor j hj hpend)).admitted p henj.1.proposalAdmitted
  have hqp : q = p := by
    have h1 := hjq.proposalAdmitted
    rw [← hviewq, hview] at h1
    exact Option.some_inj.mp (h1.symm.trans hadm)
  exact ⟨j, vote, hmem, hview, hqp ▸ hdata, hqp ▸ hepq, hsigner⟩

/--
**An owed decide is delivered**, with the view in the chain it delivers.

The shortest of the four: `DecideEnabled` reads the decide path only, which both
kinds of step keep above the floor, so the window is the floor alone.

Which blocks accompany it is `StepSpec.decideJustified`, not this: a chain may
be truncated where the node holds no ancestor, and a decide stops there rather
than waiting (see `DecideInv`).
-/
theorem decide_delivered {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {r : Run cfg (StepSpec cfg leader node)} {v : ViewNumber}
    (hfair : WeaklyFair r) (howed : DecideOwed r v) :
    ∃ j blocks c1 c2 b, Output.decided blocks c1 c2 ∈ (Run.event r j).outputs
      ∧ b ∈ blocks ∧ b.viewNumber = v := by
  obtain ⟨n, hen, hw⟩ := howed
  by_cases hex : ∃ j, n ≤ j ∧ ∃ blocks c1 c2 b,
      Output.decided blocks c1 c2 ∈ (Run.event r j).outputs ∧ b ∈ blocks ∧ b.viewNumber = v
  · obtain ⟨j, -, blocks, c1, c2, b, hmem, hb, hbv⟩ := hex
    exact ⟨j, blocks, c1, c2, b, hmem, hb, hbv⟩
  · exfalso
    have hno : ∀ i, n ≤ i → ∀ blocks c1 c2,
        Output.decided blocks c1 c2 ∈ (Run.event r i).outputs → ∀ b ∈ blocks, b.viewNumber ≠ v :=
      fun i hi blocks c1 c2 hmem b hb hbv => hex ⟨i, hi, blocks, c1, c2, b, hmem, hb, hbv⟩
    obtain ⟨j, hj, o, hmem, blocks, c1, c2, b, rfl, hb, hbv⟩ :=
      hfair.decide v n fun m hm =>
        decideEnabled_upTo hw (fun i hi _ => hno i hi) hen m hm (Nat.le_refl m)
    exact hno j hj blocks c1 c2 hmem b hb hbv

/--
**An owed proposal is sent**, in the view it was owed for.

The block is not pinned, and cannot be: two proposals differing only in
`Proposal.identity` are both justified, and a proposal a node builds has no
identity assigned yet (see `Proposal.identity`). What the specification fixes is
the view, the parent and the header, and only the view is claimed here.
-/
theorem propose_sent {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {r : Run cfg (StepSpec cfg leader node)} {p : Proposal}
    (hfair : WeaklyFair r) (howed : ProposeOwed r p) :
    ∃ j, ∃ q : Proposal, Output.send (.proposal q) ∈ (Run.event r j).outputs
      ∧ q.viewNumber = p.viewNumber := by
  obtain ⟨n, hen, hw⟩ := howed
  by_cases hex : ∃ j, n ≤ j ∧ ∃ q : Proposal,
      Output.send (.proposal q) ∈ (Run.event r j).outputs ∧ q.viewNumber = p.viewNumber
  · obtain ⟨j, -, q, hmem, hview⟩ := hex
    exact ⟨j, q, hmem, hview⟩
  · exfalso
    have hno : ∀ i, n ≤ i → ∀ q : Proposal,
        Output.send (.proposal q) ∈ (Run.event r i).outputs → q.viewNumber ≠ p.viewNumber :=
      fun i hi q hmem hview => hex ⟨i, hi, q, hmem, hview⟩
    obtain ⟨j, hj, o, hmem, q, rfl, hview⟩ :=
      hfair.propose p n fun m hm =>
        proposeEnabled_upTo hw (fun i hi _ => hno i hi) hen m hm (Nat.le_refl m)
    exact hno j hj q hmem hview

/-! ## A quorum forms the certificate

The votes of the previous section are the votes `Network.ValidCert1` and
`Network.ValidCert2` are defined as. So a quorum's worth of owed votes is a
certificate, and it is the same object the safety results reason about — which is
what `LiveNetwork.netRun` buys: the runs progress is proved over and the runs
no-fork is proved over are one set of runs, read through two relations.
-/

/--
**A quorum's vote1s for `p` are the `Cert1` over it.**

The converse of `cert1_unique`, and the first of the two results that reach past
one node. Stated over the votes rather than over the obligations, which is what
lets it meet both of the ways a vote is reached: `vote1_cast` from an owed vote,
`vote1_forced` from a delivery. Nothing says either ever happens — that is
delivery, which is not modelled — but if it does, the certificate exists, and
`Network.ValidCert1` is the form in which the rest of the specification can use
it.
-/
theorem cert1_forms {cfg : Config} {leader : ViewNumber → Option PubKey} {C : Committee}
    (N : LiveNetwork cfg leader C) {p : Proposal} {q : PubKey → Prop} (hq : C.Quorum p.epoch q)
    (hcast : ∀ k, q k → ∀ h : C.honest k, ∃ j, ∃ vote : Vote1,
      Output.send (.vote1 vote) ∈ (Run.event (N.run k h) j).outputs
        ∧ vote.view = p.viewNumber ∧ vote.data.blockHash = blockHash p ∧ vote.data.epoch = p.epoch ∧ vote.signer = k) :
    Network.ValidCert1 cfg N.net ⟨⟨blockHash p, p.epoch⟩, p.viewNumber⟩ := by
  refine ⟨q, hq, fun k hk h => ?_⟩
  obtain ⟨j, vote, hmem, hview, hdata, hsigner⟩ := hcast k hk h
  have hvote : vote = ⟨⟨blockHash p, p.epoch⟩, p.viewNumber, k⟩ := by
    cases vote with
    | mk d w s => cases d with | mk bh => simp_all
  exact hvote ▸ cast1_of_emit N k h j vote hmem

/-- **A quorum's vote2s for `p` are the `Cert2` over it.** -/
theorem cert2_forms {cfg : Config} {leader : ViewNumber → Option PubKey} {C : Committee}
    (N : LiveNetwork cfg leader C) {p : Proposal} {q : PubKey → Prop} (hq : C.Quorum p.epoch q)
    (hcast : ∀ k, q k → ∀ h : C.honest k, ∃ j, ∃ vote : Vote2,
      Output.send (.vote2 vote) ∈ (Run.event (N.run k h) j).outputs
        ∧ vote.view = p.viewNumber ∧ vote.data.blockHash = blockHash p ∧ vote.data.epoch = p.epoch ∧ vote.signer = k) :
    Network.ValidCert2 cfg N.net ⟨⟨blockHash p, p.epoch⟩, p.viewNumber⟩ := by
  refine ⟨q, hq, fun k hk h => ?_⟩
  obtain ⟨j, vote, hmem, hview, hdata, hsigner⟩ := hcast k hk h
  have hvote : vote = ⟨⟨blockHash p, p.epoch⟩, p.viewNumber, k⟩ := by
    cases vote with
    | mk d w s => cases d with | mk bh => simp_all
  exact hvote ▸ cast2_of_emit N k h j vote hmem

/--
**A quorum that owes a vote1 for `p` forms the `Cert1` over it.**

`cert1_forms` over the obligations rather than over the votes, which is the form
a delivery argument reaches first: it has nodes that owe the vote, and
`vote1_cast` is what turns each of those into the vote itself. This and
`cert2_forms_of_owed` are where `LiveNetwork.fair` is used.
-/
theorem cert1_forms_of_owed {cfg : Config} {leader : ViewNumber → Option PubKey}
    {C : Committee} (N : LiveNetwork cfg leader C) {p : Proposal} {q : PubKey → Prop}
    (hq : C.Quorum p.epoch q) (howed : ∀ k, q k → ∀ h : C.honest k, Vote1Owed (N.run k h) p) :
    Network.ValidCert1 cfg N.net ⟨⟨blockHash p, p.epoch⟩, p.viewNumber⟩ :=
  cert1_forms N hq fun k hk h => vote1_cast (N.fair k h) (howed k hk h)

/-- **A quorum that owes a vote2 for `p` forms the `Cert2` over it.** -/
theorem cert2_forms_of_owed {cfg : Config} {leader : ViewNumber → Option PubKey}
    {C : Committee} (N : LiveNetwork cfg leader C) {p : Proposal} {q : PubKey → Prop}
    (hq : C.Quorum p.epoch q) (howed : ∀ k, q k → ∀ h : C.honest k, Vote2Owed (N.run k h) p) :
    Network.ValidCert2 cfg N.net ⟨⟨blockHash p, p.epoch⟩, p.viewNumber⟩ :=
  cert2_forms N hq fun k hk h => vote2_cast (N.fair k h) (howed k hk h)

/--
**What a quorum commits lies on the one chain.**

The two halves in one statement: `cert2_forms` produces the certificate,
`decideSafety` places it. Every block committed anywhere in the network, earlier
or later, is an ancestor or a descendant of this one — so a quorum that owes a
vote2 for `p` does not merely act, it extends the single chain.

The conditions on `tree` are `decideSafety`'s, unchanged, and mean what they mean
there: without them ancestry is not a relation the statement could be about.
-/
theorem quorum_on_chain {cfg : Config} {leader : ViewNumber → Option PubKey} {C : Committee}
    (tree : BlockTable) (N : LiveNetwork cfg leader C) (hcfg : ConfigCoherent cfg)
    (htree : TreeCoherent tree) (hcf : CollisionFree) (hres : Resolves tree N.net)
    (hheights : HeightSucceedsParent tree N.net) (hroot : AnchorRooted tree cfg)
    {p : Proposal} {q : PubKey → Prop} (hq : C.Quorum p.epoch q)
    (hcast : ∀ k, q k → ∀ h : C.honest k, ∃ j, ∃ vote : Vote2,
      Output.send (.vote2 vote) ∈ (Run.event (N.run k h) j).outputs
        ∧ vote.view = p.viewNumber ∧ vote.data.blockHash = blockHash p ∧ vote.data.epoch = p.epoch ∧ vote.signer = k) :
    ∀ c, Network.ValidCert2 cfg N.net c →
      (c.view ≤ p.viewNumber → Ancestor tree c.data.blockHash (blockHash p))
        ∧ (p.viewNumber ≤ c.view → Ancestor tree (blockHash p) c.data.blockHash) :=
  fun c hc =>
    ⟨fun hle =>
        decideSafety tree N.net hcfg htree hcf hres hheights hroot c
          ⟨⟨blockHash p, p.epoch⟩, p.viewNumber⟩ hc (cert2_forms N hq hcast) hle
    , fun hle =>
        decideSafety tree N.net hcfg htree hcf hres hheights hroot
          ⟨⟨blockHash p, p.epoch⟩, p.viewNumber⟩ c (cert2_forms N hq hcast) hc hle⟩

end NewProtocol
