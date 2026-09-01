module

import NewProtocolSpec.Step

/-!
# States that owe something

Part of `NewProtocolSpec.Checks`: what that file does for the shape of the
specification, this does for its content.

`NewProtocolSpec.Deadlock` argues that each obligation's guards can be met at
once. That is a claim about the specification, so it is checked rather than
asserted: six states, and the actions they owe. A guard no state could satisfy
would leave a rule that never obliges anything, and every result about it would
be vacuous.

Two of the six exist because the other four would meet the most delicate guards
for the wrong reason. A node at the first view holds no lock and extends the
anchor, so `SafeToExtend`, the lock clause of `Vote1Enabled` and
`Vote1Justification.parentLinked` are all exempt there rather than met: the
locked witness meets them. And a first proposal carries no timeout evidence, so
the branch of `ParentCertJustified` that reads the timeout certificate and the
lock together goes untried: the witness after a timeout takes it.

What is checked is that the predicates are satisfiable, at states written down
here. That the states are *reachable* is a stronger claim and is not made.
`NewProtocolSpec.Deadlock` does not make it either: its results say what holds at
the state a step leaves, taking the step as a hypothesis. That a conforming step
relation exists at all is `NewProtocolImpl.conforms`, in the other package.

`BlockValid` is opaque, so the two vote1 witnesses take it as a hypothesis — the one
thing about a block that consensus cannot establish for itself
(`NewProtocolSpec.Assumptions`).
-/

namespace NewProtocol
namespace Checks
namespace Examples


/-- A proposal at the first view, extending the configured anchor. -/
def firstProposal (cfg : Config) : Proposal where
  blockHeader := ⟨⟨0⟩, 1⟩
  viewNumber := ⟨1⟩
  epoch := epochOf 1 cfg.epochHeight
  parentCert := cfg.anchorCert
  timeoutEvidence := none
  identity := ⟨1⟩

/-- A fresh node that has been handed that proposal, its share and its validity. -/
def admittedFirst (cfg : Config) : NodeState :=
  let p := firstProposal cfg
  { NodeState.initial cfg with
      admitted := fun v => if v = p.viewNumber then some p else none
      proposals := fun v =>
        if v = p.viewNumber then some p
        else if v = ViewNumber.genesis then some cfg.anchorBlock else none
      vidShares := fun v => if v = p.viewNumber then some ⟨p.viewNumber, p.payloadCommit⟩ else none
      validated := fun v => if v = p.viewNumber then some (blockHash p) else none }

/-- A vote1 is owed there. -/
example (cfg : Config) (hcfg : ConfigCoherent cfg) (h : BlockValid (firstProposal cfg)) :
    Vote1Enabled (admittedFirst cfg) (firstProposal cfg) := by
  have hgen : (firstProposal cfg).parentCert.view = ViewNumber.genesis := hcfg.anchorCertView
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩
  · exact { proposalAdmitted := by simp [admittedFirst]
          , blockValid := h
          , vidShare := by simp [admittedFirst]
          , safeToExtend := by simp [admittedFirst, NodeState.initial, SafeToExtend]
          , parentLinked := fun hne => absurd hgen hne }
  · simp [admittedFirst]
  · simp [admittedFirst, NodeState.initial]
  · show ((admittedFirst cfg).timeoutView).toNat < ((firstProposal cfg).viewNumber).toNat
    simp [admittedFirst, NodeState.initial, firstProposal, ViewNumber.genesis]
  · show ((admittedFirst cfg).barredView).toNat < ((firstProposal cfg).viewNumber).toNat
    simp [admittedFirst, NodeState.initial, firstProposal, ViewNumber.genesis]
  · intro lock hlock
    simp [admittedFirst, NodeState.initial] at hlock

/-- A fresh node that has been handed a block to propose at the first view. -/
def readyToPropose (cfg : Config) : NodeState :=
  let p := firstProposal cfg
  { NodeState.initial cfg with
      headers := fun v h =>
        if v = p.viewNumber ∧ h = blockHash cfg.anchorBlock then some p.blockHeader else none }

/-- A proposal is owed there. -/
example (cfg : Config) (hcfg : ConfigCoherent cfg) (node : PubKey) :
    ProposeEnabled cfg (fun _ => some node) node (readyToPropose cfg) (firstProposal cfg) := by
  refine ⟨{ leader := rfl
          , wellFormed := ⟨?_, Or.inl ?_, rfl⟩
          , justified := ?_
          , headerBuilt := ⟨cfg.anchorBlock, ?_, fun hne => absurd ?_ hne, ?_⟩ },
        by simp [readyToPropose, NodeState.initial],
        by show ((readyToPropose cfg).timeoutView).toNat < ((firstProposal cfg).viewNumber).toNat
           simp [readyToPropose, NodeState.initial, firstProposal, ViewNumber.genesis],
        by show ((readyToPropose cfg).barredView).toNat < ((firstProposal cfg).viewNumber).toNat
           simp [readyToPropose, NodeState.initial, firstProposal, ViewNumber.genesis]⟩
  · show cfg.anchorCert.view.toNat < 1
    rw [hcfg.anchorCertView]; exact Nat.zero_lt_one
  · show cfg.anchorCert.view + 1 = (⟨1⟩ : ViewNumber)
    rw [hcfg.anchorCertView]; rfl
  · show (readyToPropose cfg).cert1s ((⟨1⟩ : ViewNumber) - 1) = some cfg.anchorCert
      ∧ cfg.anchorCert.view + 1 = (⟨1⟩ : ViewNumber)
    refine ⟨?_, by rw [hcfg.anchorCertView]; rfl⟩
    simp [readyToPropose, NodeState.initial, firstProposal]
    rfl
  · show (readyToPropose cfg).proposals cfg.anchorCert.view = some cfg.anchorBlock
    rw [hcfg.anchorCertView]
    simp [readyToPropose, NodeState.initial]
  · exact hcfg.anchorCertView
  · simp [readyToPropose, firstProposal]

/-- A fresh node holding both certificates over the first block. -/
def certifiedFirst (cfg : Config) : NodeState :=
  let p := firstProposal cfg
  { NodeState.initial cfg with
      proposals := fun v =>
        if v = p.viewNumber then some p
        else if v = ViewNumber.genesis then some cfg.anchorBlock else none
      cert1s := fun v =>
        if v = p.viewNumber then some ⟨⟨blockHash p, p.epoch⟩, p.viewNumber⟩
        else if v = ViewNumber.genesis then some cfg.anchorCert else none
      cert2s := fun v => if v = p.viewNumber then some ⟨⟨blockHash p, p.epoch⟩, p.viewNumber⟩ else none }

/-- A decide is owed there. -/
example (cfg : Config) : DecideEnabled cfg (certifiedFirst cfg) (firstProposal cfg).viewNumber := by
  refine ⟨by simp [certifiedFirst, NodeState.initial, firstProposal, ViewNumber.genesis], ?_,
        by simp [certifiedFirst, firstProposal],
        ⟨⟨⟨blockHash (firstProposal cfg), (firstProposal cfg).epoch⟩, (firstProposal cfg).viewNumber⟩, firstProposal cfg,
          by simp [certifiedFirst], by simp [certifiedFirst], rfl⟩⟩
  intro w hw
  simp only [certifiedFirst, NodeState.initial] at hw
  subst hw
  show ViewNumber.genesis.toNat - cfg.decideBuffer < (firstProposal cfg).viewNumber.toNat
  show 0 - cfg.decideBuffer < 1
  omega


/--
A second proposal, extending the first rather than the anchor.

The witnesses above meet `SafeToExtend`, the lock clause of `Vote1Enabled` and
`Vote1Justification.parentLinked` vacuously — no lock is held and the parent is
genesis, which those three exempt. This one meets all three on their live
branches.
-/
def secondProposal (cfg : Config) : Proposal where
  blockHeader := ⟨⟨1⟩, 2⟩
  viewNumber := ⟨2⟩
  epoch := epochOf 2 cfg.epochHeight
  parentCert := ⟨⟨blockHash (firstProposal cfg), (firstProposal cfg).epoch⟩, ⟨1⟩⟩
  timeoutEvidence := none
  identity := ⟨2⟩

/--
A node locked on the first block, holding the second and everything it reads.

The certificate the lock names is held too: `SafetySpec.lockJustified` requires it
of the step that moves the lock, and a state without it is one no run reaches.
-/
def lockedOnFirst (cfg : Config) : NodeState :=
  { NodeState.initial cfg with
      admitted := fun v => if v = (⟨2⟩ : ViewNumber) then some (secondProposal cfg) else none
      proposals := fun v =>
        if v = (⟨2⟩ : ViewNumber) then some (secondProposal cfg)
        else if v = (⟨1⟩ : ViewNumber) then some (firstProposal cfg)
        else if v = ViewNumber.genesis then some cfg.anchorBlock else none
      vidShares := fun v =>
        if v = (⟨2⟩ : ViewNumber) then some ⟨⟨2⟩, (secondProposal cfg).payloadCommit⟩ else none
      validated := fun v =>
        if v = (⟨2⟩ : ViewNumber) then some (blockHash (secondProposal cfg)) else none
      blocksReconstructed := fun v pc =>
        v = (⟨1⟩ : ViewNumber) ∧ pc = (firstProposal cfg).payloadCommit
      cert1s := fun v =>
        if v = (⟨1⟩ : ViewNumber) then some ⟨⟨blockHash (firstProposal cfg), (firstProposal cfg).epoch⟩, ⟨1⟩⟩
        else if v = ViewNumber.genesis then some cfg.anchorCert else none
      lockedCert := some ⟨⟨blockHash (firstProposal cfg), (firstProposal cfg).epoch⟩, ⟨1⟩⟩ }

/-- A vote1 is owed there too, with the lock and the parent link both live. -/
example (cfg : Config) (h : BlockValid (secondProposal cfg)) :
    Vote1Enabled (lockedOnFirst cfg) (secondProposal cfg) := by
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩
  · exact { proposalAdmitted := by simp [lockedOnFirst, secondProposal]
          , blockValid := h
          , vidShare := by simp [lockedOnFirst, secondProposal]
          , safeToExtend := by
              show (⟨⟨blockHash (firstProposal cfg), (firstProposal cfg).epoch⟩, ⟨1⟩⟩ : Cert1).data = _ ∨ _
              exact Or.inl rfl
          , parentLinked := fun _ =>
              ⟨firstProposal cfg, by simp [lockedOnFirst, secondProposal], rfl,
               by simp [lockedOnFirst, secondProposal]⟩ }
  · simp [lockedOnFirst, secondProposal]
  · simp [lockedOnFirst, NodeState.initial]
  · show ((lockedOnFirst cfg).timeoutView).toNat < ((secondProposal cfg).viewNumber).toNat
    simp [lockedOnFirst, NodeState.initial, secondProposal, ViewNumber.genesis]
  · show ((lockedOnFirst cfg).barredView).toNat < ((secondProposal cfg).viewNumber).toNat
    simp [lockedOnFirst, NodeState.initial, secondProposal, ViewNumber.genesis]
  · intro lock hlock
    have : lock = ⟨⟨blockHash (firstProposal cfg), (firstProposal cfg).epoch⟩, ⟨1⟩⟩ := by
      simpa [lockedOnFirst] using hlock.symm
    subst this
    show (1 : Nat) < 2
    omega

/-- A node holding the `Cert1` for the first block, and its payload. -/
def certifiedOnce (cfg : Config) : NodeState :=
  { NodeState.initial cfg with
      admitted := fun v => if v = (⟨1⟩ : ViewNumber) then some (firstProposal cfg) else none
      proposals := fun v =>
        if v = (⟨1⟩ : ViewNumber) then some (firstProposal cfg)
        else if v = ViewNumber.genesis then some cfg.anchorBlock else none
      cert1s := fun v =>
        if v = (⟨1⟩ : ViewNumber) then some ⟨⟨blockHash (firstProposal cfg), (firstProposal cfg).epoch⟩, ⟨1⟩⟩
        else if v = ViewNumber.genesis then some cfg.anchorCert else none
      blocksReconstructed := fun v pc =>
        v = (⟨1⟩ : ViewNumber) ∧ pc = (firstProposal cfg).payloadCommit }

/-- A vote2 is owed there. -/
example (cfg : Config) : Vote2Enabled cfg (certifiedOnce cfg) (firstProposal cfg) := by
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_⟩
  · exact { proposalAdmitted := by simp [certifiedOnce, firstProposal]
          , certMatches := ⟨⟨⟨blockHash (firstProposal cfg), (firstProposal cfg).epoch⟩, ⟨1⟩⟩,
              by simp [certifiedOnce, firstProposal], rfl, rfl⟩
          , reconstructed := by simp [certifiedOnce, firstProposal] }
  · rintro ⟨w, u, -, hbr, -⟩
    simp [certifiedOnce, NodeState.initial] at hbr
  · simp [certifiedOnce, NodeState.initial]
  · simp [certifiedOnce, NodeState.initial]
  · simp [certifiedOnce, NodeState.initial, firstProposal, ViewNumber.genesis]
  · intro w hw
    simp only [certifiedOnce, NodeState.initial] at hw
    subst hw
    show ViewNumber.genesis.toNat - cfg.decideBuffer < (firstProposal cfg).viewNumber.toNat
    show 0 - cfg.decideBuffer < 1
    omega
  · show ((certifiedOnce cfg).barredView).toNat < ((firstProposal cfg).viewNumber).toNat
    simp [certifiedOnce, NodeState.initial, firstProposal, ViewNumber.genesis]


/--
A proposal after a timeout: the third view, extending the first.

The witnesses above take the other branch of `ParentCertJustified` throughout, so
nothing yet meets the one a timeout takes — the timeout certificate, the lock on
the parent, and the second disjunct of `ProposalWellFormed`, which want the same
state at once.
-/
def timeoutProposal (cfg : Config) : Proposal where
  blockHeader := ⟨⟨2⟩, 2⟩
  viewNumber := ⟨3⟩
  epoch := epochOf 2 cfg.epochHeight
  parentCert := ⟨⟨blockHash (firstProposal cfg), (firstProposal cfg).epoch⟩, ⟨1⟩⟩
  timeoutEvidence := some ⟨⟨epochOf 2 cfg.epochHeight⟩, ⟨2⟩⟩
  identity := ⟨3⟩

/-- A leader that timed out of the second view, holding what the third needs. -/
def afterTimeout (cfg : Config) : NodeState :=
  { NodeState.initial cfg with
      proposals := fun v =>
        if v = (⟨1⟩ : ViewNumber) then some (firstProposal cfg)
        else if v = ViewNumber.genesis then some cfg.anchorBlock else none
      cert1s := fun v =>
        if v = (⟨1⟩ : ViewNumber) then some ⟨⟨blockHash (firstProposal cfg), (firstProposal cfg).epoch⟩, ⟨1⟩⟩
        else if v = ViewNumber.genesis then some cfg.anchorCert else none
      timeoutCerts := fun v => if v = (⟨3⟩ : ViewNumber) then some ⟨⟨epochOf 2 cfg.epochHeight⟩, ⟨2⟩⟩ else none
      headers := fun v h =>
        if v = (⟨3⟩ : ViewNumber) ∧ h = blockHash (firstProposal cfg)
        then some (timeoutProposal cfg).blockHeader else none
      lockedCert := some ⟨⟨blockHash (firstProposal cfg), (firstProposal cfg).epoch⟩, ⟨1⟩⟩
      timeoutView := ⟨2⟩
      currentView := ⟨3⟩ }

/-- A proposal is owed there, on the branch a timeout takes. -/
example (cfg : Config) (node : PubKey) :
    ProposeEnabled cfg (fun _ => some node) node (afterTimeout cfg) (timeoutProposal cfg) := by
  refine ⟨{ leader := rfl
          , wellFormed := ⟨by show (1 : Nat) < 3; omega,
              Or.inr ⟨⟨⟨epochOf 2 cfg.epochHeight⟩, ⟨2⟩⟩, rfl, rfl⟩, rfl⟩
          , justified := ?_
          , headerBuilt := ⟨firstProposal cfg, by simp [afterTimeout, timeoutProposal],
              fun _ => rfl, by simp [afterTimeout, timeoutProposal]⟩ }, ?_, ?_, ?_⟩
  · show (afterTimeout cfg).timeoutCerts (⟨3⟩ : ViewNumber) = some ⟨⟨epochOf 2 cfg.epochHeight⟩, ⟨2⟩⟩
      ∧ (afterTimeout cfg).lockedCert = some (timeoutProposal cfg).parentCert
    exact ⟨by simp [afterTimeout], by simp [afterTimeout, timeoutProposal]⟩
  · simp [afterTimeout, NodeState.initial]
  · show ((afterTimeout cfg).timeoutView).toNat < ((timeoutProposal cfg).viewNumber).toNat
    show (2 : Nat) < 3
    omega
  · show ((afterTimeout cfg).barredView).toNat < ((timeoutProposal cfg).viewNumber).toNat
    simp [afterTimeout, NodeState.initial, timeoutProposal, ViewNumber.genesis]

end Examples
end Checks
end NewProtocol
