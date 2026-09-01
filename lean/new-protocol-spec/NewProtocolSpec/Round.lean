module

public import NewProtocolSpec.Round.Defs

/-!
# A round completes

The four conditional results of `NewProtocolSpec.Deadlock` and the two
certificate results of `NewProtocolSpec.Progress`, composed into the round they
are hops of: deliver a proposal to a quorum and the `Cert1` over it exists;
deliver that certificate to a quorum and the `Cert2` exists; deliver *that* to a
node holding the block and the block is decided. `round_completes` is the two
certificates together with where `decideSafety` puts the second one, and
`round_decided` is the last hop.

Each hop's conclusion is the next hop's hypothesis as the same object, not as a
copy: the certificate `cert1_forms` produces is the one `Vote2Delivery.certArrival`
delivers, and likewise for `Cert2`. That is the whole point of composing them,
and it is what `LiveNetwork.netRun` makes possible, since the certificates the
progress results build are the certificates the safety results govern.

What is *not* claimed, and cannot be. Nothing says a delivery happens: every hop
takes its arrivals as hypotheses, so a round is a statement about what follows
from delivery rather than a promise of it. Nothing carries a node's state from
one hop to the next either — the steps between two deliveries are unconstrained,
so `Vote2Delivery.admitted` is a hypothesis where an eye reading the round in
order would expect a consequence.

The leader's proposal is not a hypothesis of any of this, and folding it in would
misrepresent what is proved. `propose_sent` says a leader that owes a proposal
sends one, but a proposal *sent* and a proposal *arriving* at a node are joined
only by delivery, which the specification does not model. So the round starts
where the arrivals do.
-/

@[expose] public section

namespace NewProtocol

/-! ## Each member of the quorum acts

Two steps from a delivery to a vote, taken once per member. Stated apart from the
certificate results because `quorum_on_chain` wants the votes rather than the
certificate they form.
-/

/-- **Every member of the quorum handed the proposal casts a vote1 for it.** -/
theorem vote1_cast_of_delivered {cfg : Config} {leader : ViewNumber → Option PubKey}
    {C : Committee} (N : LiveNetwork cfg leader C) {q : PubKey → Prop} {p : Proposal}
    (hd : ∀ k, q k → ∀ h : C.honest k, Vote1Delivered (N.run k h) p) :
    ∀ k, q k → ∀ h : C.honest k, ∃ j, ∃ vote : Vote1,
      Output.send (.vote1 vote) ∈ (Run.event (N.run k h) j).outputs
        ∧ vote.view = p.viewNumber ∧ vote.data.blockHash = blockHash p ∧ vote.data.epoch = p.epoch ∧ vote.signer = k := by
  intro k hk h
  obtain ⟨n, n₂, d⟩ := hd k hk h
  obtain ⟨sender, vid, harrival, hadmissible⟩ := d.arrival
  exact vote1_forced (N.fair k h) harrival d.validated d.order hadmissible d.parentHeld
    d.valid d.writable d.fresh d.window

/-- **Every member of the quorum handed the `Cert1` casts a vote2 for the block.** -/
theorem vote2_cast_of_delivered {cfg : Config} {leader : ViewNumber → Option PubKey}
    {C : Committee} (N : LiveNetwork cfg leader C) {q : PubKey → Prop} {p : Proposal}
    (hd : ∀ k, q k → ∀ h : C.honest k, Vote2Delivered (N.run k h) p) :
    ∀ k, q k → ∀ h : C.honest k, ∃ j, ∃ vote : Vote2,
      Output.send (.vote2 vote) ∈ (Run.event (N.run k h) j).outputs
        ∧ vote.view = p.viewNumber ∧ vote.data.blockHash = blockHash p ∧ vote.data.epoch = p.epoch ∧ vote.signer = k := by
  intro k hk h
  obtain ⟨n, n₂, d⟩ := hd k hk h
  exact vote2_forced (N.fair k h) d.certArrival d.payloadArrival d.order d.admitted rfl rfl rfl
    d.writable d.fresh d.window

/-! ## The two hops that build a certificate -/

/--
**A quorum handed the proposal forms the `Cert1` over it.**

The first hop. `cert1_forms` turns the votes into the certificate, and the
certificate is `Vote2Delivery.certArrival`'s.
-/
theorem round_cert1 {cfg : Config} {leader : ViewNumber → Option PubKey} {C : Committee}
    (N : LiveNetwork cfg leader C) {p : Proposal} {q : PubKey → Prop} (hq : C.Quorum p.epoch q)
    (hd : ∀ k, q k → ∀ h : C.honest k, Vote1Delivered (N.run k h) p) :
    Network.ValidCert1 cfg N.net ⟨⟨blockHash p, p.epoch⟩, p.viewNumber⟩ :=
  cert1_forms N hq (vote1_cast_of_delivered N hd)

/--
**A quorum handed the `Cert1` forms the `Cert2` over the same block.**

The second hop, and the one the protocol exists to reach: a block with a `Cert2`
is decided.
-/
theorem round_cert2 {cfg : Config} {leader : ViewNumber → Option PubKey} {C : Committee}
    (N : LiveNetwork cfg leader C) {p : Proposal} {q : PubKey → Prop} (hq : C.Quorum p.epoch q)
    (hd : ∀ k, q k → ∀ h : C.honest k, Vote2Delivered (N.run k h) p) :
    Network.ValidCert2 cfg N.net ⟨⟨blockHash p, p.epoch⟩, p.viewNumber⟩ :=
  cert2_forms N hq (vote2_cast_of_delivered N hd)

/-! ## The round -/

/--
**A round of delivery commits the block, on the one chain.**

The headline: two quorums, the first handed the proposal and the second handed
the `Cert1` those first votes formed, and the block is committed and placed
against every other commitment the network makes, earlier or later.

The quorums are separate and need not overlap beyond what `Committee.intersect`
already forces. Nothing about the round needs them to be the same, and requiring
it would state something weaker than what holds.

The third conjunct is `quorum_on_chain`, which is `decideSafety` applied to the
certificate the second conjunct builds. Its conditions on `tree` are that
result's, unchanged, and mean what they mean there: without them ancestry is not
a relation the statement could be about.
-/
theorem round_completes {cfg : Config} {leader : ViewNumber → Option PubKey} {C : Committee}
    (tree : BlockTable) (N : LiveNetwork cfg leader C) (hcfg : ConfigCoherent cfg)
    (htree : TreeCoherent tree) (hcf : CollisionFree) (hres : Resolves tree N.net)
    (hheights : HeightSucceedsParent tree N.net)
    {p : Proposal} {q₁ q₂ : PubKey → Prop} (hq₁ : C.Quorum p.epoch q₁) (hq₂ : C.Quorum p.epoch q₂)
    (hd₁ : ∀ k, q₁ k → ∀ h : C.honest k, Vote1Delivered (N.run k h) p)
    (hd₂ : ∀ k, q₂ k → ∀ h : C.honest k, Vote2Delivered (N.run k h) p) :
    Network.ValidCert1 cfg N.net ⟨⟨blockHash p, p.epoch⟩, p.viewNumber⟩
      ∧ Network.ValidCert2 cfg N.net ⟨⟨blockHash p, p.epoch⟩, p.viewNumber⟩
      ∧ ∀ c, Network.ValidCert2 cfg N.net c →
          (c.view ≤ p.viewNumber → Ancestor tree c.data.blockHash (blockHash p))
            ∧ (p.viewNumber ≤ c.view → Ancestor tree (blockHash p) c.data.blockHash) :=
  ⟨round_cert1 N hq₁ hd₁, round_cert2 N hq₂ hd₂,
    quorum_on_chain tree N hcfg htree hcf hres hheights hq₂
      (vote2_cast_of_delivered N hd₂)⟩

/--
**And a node handed the resulting `Cert2` decides that view.**

The last hop, kept apart from `round_completes` because it is about one node
rather than the network: the certificate a quorum formed is delivered to a node
that holds the block, and the decide follows. The two are joined by the
certificate being one object — `DecideDelivery.arrival` names what `round_cert2`
produces.

Which blocks accompany the decide is `StepSpec.decideJustified`, not this: the
chain is truncated where the node holds no ancestor, so all that is claimed of
the delivered chain is that the view is in it (see `DecideInv`).
-/
theorem round_decided {cfg : Config} {leader : ViewNumber → Option PubKey} {C : Committee}
    (N : LiveNetwork cfg leader C) {p : Proposal} {q : PubKey → Prop} (hq : C.Quorum p.epoch q)
    (hd : ∀ k, q k → ∀ h : C.honest k, Vote2Delivered (N.run k h) p)
    {node : PubKey} (hnode : C.honest node) (hdec : DecideDelivered (N.run node hnode) p) :
    Network.ValidCert2 cfg N.net ⟨⟨blockHash p, p.epoch⟩, p.viewNumber⟩
      ∧ ∃ j blocks c1 c2 b,
          Output.decided blocks c1 c2 ∈ (Run.event (N.run node hnode) j).outputs
            ∧ b ∈ blocks ∧ b.viewNumber = p.viewNumber := by
  obtain ⟨n, d⟩ := hdec
  exact ⟨round_cert2 N hq hd,
    decide_forced (N.fair node hnode) d.arrival rfl d.cert1Held d.blockHeld rfl d.writable
      d.fresh d.window⟩

end NewProtocol
