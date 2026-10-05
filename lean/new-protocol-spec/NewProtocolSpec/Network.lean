module

public import NewProtocolSpec.Rules

/-!
# Networks

Honest nodes and what connects them: one trace per honest node, obeying the
signing rules, and certificates that mean a quorum really voted.

Faulty nodes have no trace. Nothing constrains what they send, which is how they
are modelled.

The network is authenticated and encrypted, so a node knows who sent it a
message. Only votes need signatures: they are aggregated into certificates that
travel on to nodes that never saw the votes. That a certificate an honest node is
handed stands for real votes is one of the two cryptographic assumptions, and it
is the content of `Network.cert1Genuine`, `Network.cert2Genuine`,
`Network.timeoutCertGenuine` and `Network.revoteGenuine`. The other is that block
hashes do not collide (`CollisionFree`).
-/

@[expose] public section

namespace NewProtocol

/--
The committees, one per epoch, as quorum systems.

`Committee.intersect` is what safety takes from stake: two quorums of one epoch
share a member honest in that epoch. `Committee.honestFinite` is for liveness.

Honesty is per epoch. A node honest in epoch `e` signs every message of `e` by
the rules, whenever it sends it; what it signs for an epoch it is not honest in
is unconstrained. So a node may be honest in one epoch and faulty in the next.
That a node later faulty cannot sign messages of an epoch it was honest in, with
that epoch's keys, is part of the assumption.
-/
structure Committee where
  /--
  The nodes that follow the rules in an epoch.

  A node may be honest in an epoch without being a member of its committee
  (`members`). It still receives the epoch's messages and decides. Its votes are
  not meant to count towards a quorum of the epoch, though `Committee.Quorum` does not
  require its signers to be members: a set holding a quorum is one too.
  -/
  honest : EpochNumber → PubKey → Prop

  /-- The members of an epoch's committee. -/
  members : EpochNumber → PubKey → Prop

  /-- The sets of signers that suffice for a certificate in an epoch. -/
  Quorum : EpochNumber → (PubKey → Prop) → Prop

  /-- Two quorums of one epoch share a member honest in that epoch. -/
  intersect : ∀ e q q', Quorum e q → Quorum e q' → ∃ k, q k ∧ q' k ∧ honest e k

  /--
  Finitely many nodes are honest in some epoch.

  Over all epochs, not per epoch: a run whose validators keep changing for ever,
  each honest in some epoch, is not covered. Liveness reads it to bound what all
  honest nodes decided by a time.
  -/
  honestFinite : ∃ ks : List PubKey, ∀ e k, honest e k → k ∈ ks

/--
The node is honest in some epoch.

These are the nodes the model records a trace of. A node honest in no epoch has
none: nothing constrains what it sends.
-/
def Committee.Honest (C : Committee) (k : PubKey) : Prop := ∃ e, C.honest e k

/-- A node honest in an epoch is honest in some epoch. -/
theorem Committee.Honest.of {C : Committee} {e : EpochNumber} {k : PubKey} (h : C.honest e k) :
    C.Honest k :=
  ⟨e, h⟩

/--
The node is honest in epoch `e` or a later one.

Not that it is honest in every epoch from `e` on: one later epoch suffices. The
network delivers a message of epoch `e` to these nodes only (`Synchrony`). A
node honest in earlier epochs only has retired by the time `e` matters.
-/
def Committee.HonestFrom (C : Committee) (e : EpochNumber) (k : PubKey) : Prop :=
  ∃ e', e.toNat ≤ e'.toNat ∧ C.honest e' k

/-- A node honest in an epoch is honest in it or a later one. -/
theorem Committee.HonestFrom.of {C : Committee} {e : EpochNumber} {k : PubKey} (h : C.honest e k) :
    C.HonestFrom e k :=
  ⟨e, Nat.le_refl _, h⟩

/-- Honest from an epoch on is honest from any earlier epoch on. -/
theorem Committee.HonestFrom.mono {C : Committee} {e e' : EpochNumber} {k : PubKey} (hle : e.toNat ≤ e'.toNat)
    (h : C.HonestFrom e' k) : C.HonestFrom e k :=
  let ⟨e'', h1, h2⟩ := h; ⟨e'', Nat.le_trans hle h1, h2⟩

/-- A node honest from an epoch on is honest in some epoch. -/
theorem Committee.HonestFrom.honest {C : Committee} {e : EpochNumber} {k : PubKey} (h : C.HonestFrom e k) :
    C.Honest k :=
  let ⟨e', _, h'⟩ := h; ⟨e', h'⟩

/--
The node is honest in infinitely many epochs: in one at or after every epoch.

When epochs have blocks, `ChainGrows` promises such a node keeps deciding.
-/
def Committee.HonestOften (C : Committee) (k : PubKey) : Prop := ∀ e, C.HonestFrom e k

/--
The node is honest in every epoch.

`ChainGrows` promises these nodes keep deciding themselves, at any epoch height. A
node honest in some epochs only is held to the protocol in those, and is not
promised to decide in the others.
-/
def Committee.Steady (C : Committee) (k : PubKey) : Prop := ∀ e, C.honest e k

/-- A node's steps, one per index, for ever. -/
abbrev Trace := Nat → Step

/-- The first `n` steps of a trace. -/
def Trace.history (r : Trace) (n : Nat) : History := (List.range n).map r

/-- The trace's node sent `m` at some step. -/
def SentBy (r : Trace) (m : Message) : Prop := ∃ n, Output.send m ∈ (r n).output

/--
`vote` is a timeout vote by `k` behind `tc`: for its view and epoch, with a lock no
later than `tc`'s.
-/
def TimeoutVoteFor (k : PubKey) (tc : TimeoutCert) (vote : TimeoutVote) : Prop :=
  vote.signer = k ∧ vote.view = tc.view ∧ vote.data.epoch = tc.data.epoch
    ∧ LockLE vote.data.lock tc.data.lock

section Backed

variable {C : Committee} (trace : ∀ k, C.Honest k → Trace)

/-- A quorum of `c`'s epoch cast the vote1s behind `c`. Only its honest members are held to it. -/
def Cert1Backed (c : Cert1) : Prop :=
  ∃ q, C.Quorum c.data.epoch q ∧ ∀ k, q k → ∀ h : C.honest c.data.epoch k,
    SentBy (trace k (.of h)) (.vote1 ⟨c.data, c.view, k⟩)

/-- A quorum of `c`'s epoch cast the vote2s behind `c`. -/
def Cert2Backed (c : Cert2) : Prop :=
  ∃ q, C.Quorum c.data.epoch q ∧ ∀ k, q k → ∀ h : C.honest c.data.epoch k,
    SentBy (trace k (.of h)) (.vote2 ⟨c.data, c.view, k⟩)

/-- A quorum of `tc`'s epoch timed out on its view, each with a lock no later than `tc`'s. -/
def TimeoutCertBacked (tc : TimeoutCert) : Prop :=
  ∃ q, C.Quorum tc.data.epoch q ∧ ∀ k, q k → ∀ h : C.honest tc.data.epoch k,
    ∃ vote, TimeoutVoteFor k tc vote ∧ SentBy (trace k (.of h)) (.timeoutVote vote)

/--
The lock a timeout certificate names passed the verifier: it is the anchor's or
backed, and no later than the view timed out.

An honest signer's lock is at an earlier view than the one it times out, so the
latest of the signers' locks is too.
-/
def TimeoutLockChecked (cfg : Config) (tc : TimeoutCert) : Prop :=
  (tc.data.lock = cfg.anchorCert ∨ Cert1Backed trace tc.data.lock) ∧ tc.data.lock.view ≤ tc.view

end Backed

/--
One trace per node honest in some epoch, obeying the signing rules for the epochs
it is honest in, where every certificate a traced node is handed stands for real
votes.

A node's inputs are what reached it, whichever epochs it is honest in:
certificates cannot be forged. The anchor's certificate is the exception: nothing
votes at genesis, and the configuration vouches for it (`ConfigCoherent`).
-/
structure Network (cfg : Config) (C : Committee) where
  /-- One trace per node honest in some epoch. -/
  trace : ∀ k, C.Honest k → Trace

  /-- Every prefix obeys the signing rules, for the epochs the node is honest in. -/
  safe : ∀ k h n, SafeHistory cfg k (C.honest · k) ((trace k h).history n)

  /-- Every `Cert1` an input carries is backed, or is the anchor's. -/
  cert1Genuine : ∀ k h n c, c ∈ (trace k h n).input.cert1 → c = cfg.anchorCert ∨ Cert1Backed trace c

  /-- Every `Cert2` an input carries is backed. -/
  cert2Genuine : ∀ k h n c, c ∈ (trace k h n).input.cert2 → Cert2Backed trace c

  /--
  Every timeout certificate an input carries is backed, and its lock checked.

  The verifier checks the certificate each signer names as its lock, so the latest
  of them is a real `Cert1`. That is what lets every honest node fetch its
  proposal and payload (`Synchrony.timeoutLockSpread`).
  -/
  timeoutCertGenuine : ∀ k h n tc, tc ∈ (trace k h n).input.timeoutCert →
    TimeoutCertBacked trace tc ∧ TimeoutLockChecked trace cfg tc

  /--
  The certificate a re-vote request votes on again is backed.

  It is over the last block of an epoch, never the anchor, so the verifier checks
  it as it checks any `Cert1`.
  -/
  revoteGenuine : ∀ k h n sender r, (trace k h n).input = .revote sender r →
    Cert1Backed trace r.cert

end NewProtocol
