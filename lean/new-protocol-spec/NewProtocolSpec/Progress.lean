module

public import NewProtocolSpec.Progress.Defs
public import NewProtocolSpec.Progress.Lemmas
public import NewProtocolSpec.Safety

/-!
# Conditional progress

What an owed action is worth. Each result here takes an action owed inside a
window (`Vote1Owed` and its three companions) and concludes that the action is
*taken*: the step that consumes `WeaklyFair`, and the only place in the
specification where an obligation turns into an output.

None of this is liveness. Nothing here says a window ever opens — that needs the
delivery and timing assumptions the specification does not have — so every
result is conditional on the environment having done its part. What they do is
establish that the rules leave no way to stall once it has: the enabledness
predicates are stable under everything a node may do to itself, and the mark
obligations leave no way to retire an action in silence.

Two of them cross from one node to the network. `cert1_forms` and `cert2_forms`
turn a quorum's worth of owed votes into a certificate — and into the *same*
certificate the safety results speak about, which is what `LiveNetwork.netRun`
is for. `quorum_on_chain` is the two halves in one statement: what progress
produces, no-fork governs.
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
which is exactly `WeaklyFair`'s antecedent, and its conclusion contradicts the
supposition — a vote cast before the window opened would not, which is what
`Run.EmitsFrom` rules out.

The block is read off the step that cast the vote, which is why the *first* such
step is the one taken: after it the mark is set, and a window says nothing about
a view the node has already voted in. `SafetySpec.vote1Justified` says the vote
signs whatever the node has admitted at that view, and
`StepSpec.contentRetained` — which is not keyed on the bar — says that is still
`p` when the step ends.
-/
theorem vote1_cast {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {r : Run cfg (StepSpec cfg leader node)} {p : Proposal}
    (hfair : WeaklyFair r) (howed : Vote1Owed r p) :
    ∃ j, ∃ vote : Vote1, Output.send (.vote1 vote) ∈ (Run.event r j).outputs
      ∧ vote.view = p.viewNumber ∧ vote.data.blockHash = blockHash p ∧ vote.signer = node := by
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
  obtain ⟨q, hjq, hviewq, hsigner, hdata⟩ := SafetySpec.vote1Justified hs.toSafetySpec vote hin
  have hadm : (Run.state r (j + 1)).admitted p.viewNumber = some p :=
    (retainsVote_of_step hs (hw.floor j hj hpend)).admitted p henj.1.proposalAdmitted
  have hqp : q = p := by
    have h1 := hjq.proposalAdmitted
    rw [← hviewq, hview] at h1
    exact Option.some_inj.mp (h1.symm.trans hadm)
  exact ⟨j, vote, hmem, hview, hqp ▸ hdata, hsigner⟩

/--
**An owed vote2 is cast**, for the block the node admitted.

As `vote1_cast`, with `SafetySpec.vote2Justified` reading the block.
-/
theorem vote2_cast {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {r : Run cfg (StepSpec cfg leader node)} {p : Proposal}
    (hfair : WeaklyFair r) (howed : Vote2Owed r p) :
    ∃ j, ∃ vote : Vote2, Output.send (.vote2 vote) ∈ (Run.event r j).outputs
      ∧ vote.view = p.viewNumber ∧ vote.data.blockHash = blockHash p ∧ vote.signer = node := by
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
  obtain ⟨q, hjq, hviewq, hsigner, hdata⟩ := SafetySpec.vote2Justified hs.toSafetySpec vote hin
  have hadm : (Run.state r (j + 1)).admitted p.viewNumber = some p :=
    (retainsVote_of_step hs (hw.floor j hj hpend)).admitted p henj.1.proposalAdmitted
  have hqp : q = p := by
    have h1 := hjq.proposalAdmitted
    rw [← hviewq, hview] at h1
    exact Option.some_inj.mp (h1.symm.trans hadm)
  exact ⟨j, vote, hmem, hview, hqp ▸ hdata, hsigner⟩

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
**A quorum that owes a vote1 for `p` forms the `Cert1` over it.**

The converse of `cert1_unique`, and the first of the two results that reach past
one node. Nothing says the quorum's members ever come to owe the vote — that is
delivery, which is not modelled — but if they do, the certificate exists, and
`Network.ValidCert1` is the form in which the rest of the specification can use
it.
-/
theorem cert1_forms {cfg : Config} {leader : ViewNumber → Option PubKey} {C : Committee}
    (N : LiveNetwork cfg leader C) {q : PubKey → Prop} (hq : C.Quorum q) {p : Proposal}
    (howed : ∀ k, q k → ∀ h : C.honest k, Vote1Owed (N.run k h) p) :
    Network.ValidCert1 cfg N.net ⟨⟨blockHash p⟩, p.viewNumber⟩ := by
  refine ⟨q, hq, fun k hk h => ?_⟩
  obtain ⟨j, vote, hmem, hview, hdata, hsigner⟩ := vote1_cast (N.fair k h) (howed k hk h)
  have hvote : vote = ⟨⟨blockHash p⟩, p.viewNumber, k⟩ := by
    cases vote with
    | mk d w s => cases d with | mk bh => simp_all
  exact hvote ▸ cast1_of_emit N k h j vote hmem

/-- **A quorum that owes a vote2 for `p` forms the `Cert2` over it.** -/
theorem cert2_forms {cfg : Config} {leader : ViewNumber → Option PubKey} {C : Committee}
    (N : LiveNetwork cfg leader C) {q : PubKey → Prop} (hq : C.Quorum q) {p : Proposal}
    (howed : ∀ k, q k → ∀ h : C.honest k, Vote2Owed (N.run k h) p) :
    Network.ValidCert2 cfg N.net ⟨⟨blockHash p⟩, p.viewNumber⟩ := by
  refine ⟨q, hq, fun k hk h => ?_⟩
  obtain ⟨j, vote, hmem, hview, hdata, hsigner⟩ := vote2_cast (N.fair k h) (howed k hk h)
  have hvote : vote = ⟨⟨blockHash p⟩, p.viewNumber, k⟩ := by
    cases vote with
    | mk d w s => cases d with | mk bh => simp_all
  exact hvote ▸ cast2_of_emit N k h j vote hmem

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
    {q : PubKey → Prop} (hq : C.Quorum q) {p : Proposal}
    (howed : ∀ k, q k → ∀ h : C.honest k, Vote2Owed (N.run k h) p) :
    ∀ c, Network.ValidCert2 cfg N.net c → c.view ≤ p.viewNumber →
      Ancestor tree c.data.blockHash (blockHash p) :=
  fun c hc hle =>
    decideSafety tree N.net hcfg htree hcf hres c ⟨⟨blockHash p⟩, p.viewNumber⟩ hc
      (cert2_forms N hq howed) hle

end NewProtocol
