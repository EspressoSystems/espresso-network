module

public import NewProtocolSpec.Proofs.Traces

/-!
# What an honest node's votes say

The signing rules of `SafeHistory`, read off a whole trace, for votes of epochs the
node is honest in: every such vote is for something it received, at most one per view, a vote1 only on a
safe parent, and a timeout vote's lock covers the vote2s before it. Everything
the safety argument takes from a node is here.
-/

@[expose] public section

namespace NewProtocol

variable {cfg : Config} {C : Committee} (N : Network cfg C)

/-- A history that signs by the rules in every epoch signs by them in any set of epochs. -/
theorem SafeHistory.of_every {node : PubKey} {P : EpochNumber → Prop} {h : History}
    (hs : SafeHistory cfg node (fun _ => True) h) : SafeHistory cfg node P h where
  vote1Justified := fun n vote ⟨st, hn, hm⟩ _ => hs.vote1Justified n vote ⟨st, hn, hm⟩ trivial
  vote1Once v v' a b _ he hv := hs.vote1Once v v' a b trivial he hv
  vote2Justified := fun n vote ⟨st, hn, hm⟩ _ => hs.vote2Justified n vote ⟨st, hn, hm⟩ trivial
  vote2Once v v' a b _ he hv := hs.vote2Once v v' a b trivial he hv
  vote2BeforeTimeout := fun n vote ⟨st, hn, hm⟩ _ tv htv _ => hs.vote2BeforeTimeout n vote ⟨st, hn, hm⟩ trivial tv htv trivial
  timeoutLock := fun n vote ⟨st, hn, hm⟩ _ v2 hv2 _ => hs.timeoutLock n vote ⟨st, hn, hm⟩ trivial v2 hv2 trivial
  decideJustified n st blocks c1 c2 hn hm _ := hs.decideJustified n st blocks c1 c2 hn hm trivial
  decideOnce n st i blocks c1 c2 hn hi _ := hs.decideOnce n st i blocks c1 c2 hn hi trivial

/-- The prefix of a trace through step `n`, read through the signing rules. -/
theorem Network.safe_at (k : PubKey) (h : C.Honest k) (n : Nat) :
    SafeHistory cfg k (C.honest · k) ((N.trace k h).history (n + 1)) := N.safe k h (n + 1)

/-- The parent certificate of a proposal an honest node is handed is backed, or the anchor's. -/
theorem Network.parentGenuine (k : PubKey) (h : C.Honest k) (n : Nat) (sender : PubKey) (p : Proposal)
    (vid : VidShare) (hin : (N.trace k h n).input = .proposal sender p (some vid)) :
    p.parentCert = cfg.anchorCert ∨ Cert1Backed N.trace p.parentCert :=
  N.cert1Genuine k h n _ (by rw [hin]; rfl)

/-- The timeout evidence of a proposal an honest node is handed is backed, and its lock checked. -/
theorem Network.evidenceGenuine (k : PubKey) (h : C.Honest k) (n : Nat) (sender : PubKey)
    (p : Proposal) (vid : VidShare) (tc : TimeoutCert) (hin : (N.trace k h n).input = .proposal sender p (some vid))
    (hte : p.timeoutEvidence = some tc) :
    TimeoutCertBacked N.trace tc ∧ TimeoutLockChecked N.trace cfg tc :=
  N.timeoutCertGenuine k h n tc (by rw [hin]; simp [Input.timeoutCert, hte])

/-- The timeout evidence of a re-vote request an honest node is handed is backed, and its lock checked. -/
theorem Network.revoteEvidenceGenuine (k : PubKey) (h : C.Honest k) (n : Nat) (sender : PubKey)
    (r : RevoteRequest) (tc : TimeoutCert) (hin : (N.trace k h n).input = .revote sender r)
    (hte : r.timeoutEvidence = some tc) :
    TimeoutCertBacked N.trace tc ∧ TimeoutLockChecked N.trace cfg tc :=
  N.timeoutCertGenuine k h n tc (by rw [hin]; simp [Input.timeoutCert, hte])

/-- A timeout certificate an honest node is handed has a checked lock. -/
theorem Network.timeoutLockGenuine (k : PubKey) (h : C.Honest k) (n : Nat) (tc : TimeoutCert)
    (hin : (N.trace k h n).input = .timeoutCertificate tc) : TimeoutLockChecked N.trace cfg tc :=
  (N.timeoutCertGenuine k h n tc (by rw [hin]; rfl)).2

/-- A `Cert2` an honest node holds is backed. -/
theorem cert2_held_backed {k : PubKey} {h : C.Honest k} {n : Nat} {c : Cert2}
    (hc : ((N.trace k h).history n).HasCert2 c) : Cert2Backed N.trace c := by
  rcases hc with hrec | ⟨c1, p, hrec⟩
  · obtain ⟨m, -, hm⟩ := (Trace.received_history _).mp hrec
    exact N.cert2Genuine k h m c (by rw [hm]; rfl)
  · obtain ⟨m, -, hm⟩ := (Trace.received_history _).mp hrec
    exact N.cert2Genuine k h m c (by rw [hm]; rfl)

/--
A vote1 an honest node sends is its own, for a well-formed valid proposal it
received at some step, or answers a well-formed re-vote request it received. A
proposal opening an epoch comes behind a `Cert2` over its parent, at an earlier
view, which is backed or the anchor's.
-/
theorem vote1_signed {k : PubKey} {h : C.Honest k} {v : Vote1}
    (hs : SentBy (N.trace k h) (.vote1 v)) (he : C.honest v.data.epoch k) :
    v.signer = k
      ∧ ((∃ m sender p vid, (N.trace k h m).input = .proposal sender p (some vid)
          ∧ ProposalWellFormed cfg p ∧ BlockValid p
          ∧ (EntersEpoch cfg p → ∃ bc : Cert2, (Cert2Backed N.trace bc ∨ bc = cfg.anchorCert2) ∧ bc.view < p.viewNumber
            ∧ bc.data = p.parentCert.data.toVote2)
          ∧ Vote1For v p)
        ∨ ∃ m sender r, (N.trace k h m).input = .revote sender r
          ∧ RevoteWellFormed cfg r ∧ Vote1Again v r) := by
  obtain ⟨n, hn⟩ := hs
  obtain ⟨hsig, hcase⟩ :=
    (N.safe_at k h n).vote1Justified n v ⟨(N.trace k h n), (Trace.history_getElem? _ (Nat.lt_succ_self n)), hn⟩ he
  rw [Trace.history_upTo _ (Nat.le_refl _)] at hcase
  refine ⟨hsig, ?_⟩
  rcases hcase with ⟨sender, p, vid, hrec, hwf, hval, -, hop, hfor⟩ | ⟨sender, r, hrec, hwf, -, hfor⟩
  · obtain ⟨m, -, hm⟩ := (Trace.received_history _).mp hrec
    refine Or.inl ⟨m, sender, p, vid, hm, hwf, hval, fun hent => ?_, hfor⟩
    obtain ⟨-, bc, hbc, hbv, hbd⟩ := hop hent
    exact ⟨bc, hbc.imp_left (cert2_held_backed N), hbv, hbd⟩
  · obtain ⟨m, -, hm⟩ := (Trace.received_history _).mp hrec
    exact Or.inr ⟨m, sender, r, hm, hwf, hfor⟩

/-- A node's vote1s at one epoch and view, of an epoch it is honest in, are one vote. -/
theorem vote1_agree {k : PubKey} {h : C.Honest k} {v v' : Vote1}
    (hs : SentBy (N.trace k h) (.vote1 v)) (hs' : SentBy (N.trace k h) (.vote1 v'))
    (he : C.honest v.data.epoch k) (heq : v.data.epoch = v'.data.epoch)
    (hv : v.view = v'.view) : v = v' := by
  obtain ⟨n, hn, -⟩ := Trace.sent_of_sentBy _ hs
  obtain ⟨n', hn', -⟩ := Trace.sent_of_sentBy _ hs'
  exact (N.safe k h (max n n' + 1)).vote1Once v v'
    (Trace.sent_mono _ (Nat.succ_le_succ (Nat.le_max_left n n')) hn)
    (Trace.sent_mono _ (Nat.succ_le_succ (Nat.le_max_right n n')) hn') he heq hv

/--
A vote2 an honest node sends is its own, after genesis, and is over what a `Cert1`
it held names, at that certificate's view.
-/
theorem vote2_signed {k : PubKey} {h : C.Honest k} {v : Vote2}
    (hs : SentBy (N.trace k h) (.vote2 v)) (he : C.honest v.data.epoch k) :
    v.signer = k ∧ cfg.anchorView < v.view ∧ ∃ c, v.view = c.view ∧ v.data = c.data.toVote2
      ∧ ∃ n, ((N.trace k h).history (n + 1)).HasCert1 cfg c := by
  obtain ⟨n, hn⟩ := hs
  obtain ⟨hsig, hgen, c, b, hc, -, -, -, hview, hdata⟩ :=
    (N.safe_at k h n).vote2Justified n v ⟨(N.trace k h n), (Trace.history_getElem? _ (Nat.lt_succ_self n)), hn⟩ he
  rw [Trace.history_upTo _ (Nat.le_refl _)] at hc
  exact ⟨hsig, hgen, c, hview, hdata, n, hc⟩

/--
A vote1 an honest node sends is on a proposal it received with a safe parent, which
names its parent at the parent's view if it opens an epoch, or answers a safe
re-vote request it received.
-/
theorem vote1_safe {k : PubKey} {h : C.Honest k} {v : Vote1}
    (hs : SentBy (N.trace k h) (.vote1 v)) (he : C.honest v.data.epoch k) :
    (∃ m sender p vid, (N.trace k h m).input = .proposal sender p (some vid)
        ∧ ProposalWellFormed cfg p ∧ SafeParent p
        ∧ (∃ n, OpensEpochJustified cfg ((N.trace k h).history n) p) ∧ Vote1For v p)
      ∨ ∃ m sender r, (N.trace k h m).input = .revote sender r
        ∧ RevoteWellFormed cfg r ∧ SafeRevote r ∧ Vote1Again v r := by
  obtain ⟨n, hn⟩ := hs
  obtain ⟨-, hcase⟩ :=
    (N.safe_at k h n).vote1Justified n v ⟨(N.trace k h n), (Trace.history_getElem? _ (Nat.lt_succ_self n)), hn⟩ he
  rw [Trace.history_upTo _ (Nat.le_refl _)] at hcase
  rcases hcase with ⟨sender, p, vid, hrec, hwf, -, hsafe, hopen, hfor⟩
    | ⟨sender, r, hrec, hwf, hsafe, hfor⟩
  · obtain ⟨m, -, hm⟩ := (Trace.received_history _).mp hrec
    exact Or.inl ⟨m, sender, p, vid, hm, hwf, hsafe, ⟨n + 1, hopen⟩, hfor⟩
  · obtain ⟨m, -, hm⟩ := (Trace.received_history _).mp hrec
    exact Or.inr ⟨m, sender, r, hm, hwf, hsafe, hfor⟩

/--
An honest node's timeout vote for a view names a lock no earlier, in lock order,
than any vote2 the node cast at a view up to it.

The vote2 came first, since none follows a timeout of its view, and the timeout
vote's lock covers every vote2 before it.
-/
theorem timeout_lock_covers {k : PubKey} {h : C.Honest k} {v2 : Vote2} {tv : TimeoutVote}
    (hs2 : SentBy (N.trace k h) (.vote2 v2)) (hst : SentBy (N.trace k h) (.timeoutVote tv))
    (he2 : C.honest v2.data.epoch k) (het : C.honest tv.data.epoch k) (hle : v2.view ≤ tv.view) :
    EpochViewLE v2.data.epoch v2.view tv.data.lock.data.epoch tv.data.lock.view := by
  obtain ⟨i, hi⟩ := hs2
  obtain ⟨j, hj⟩ := hst
  by_cases hji : j ≤ i
  · exfalso
    have hlt := (N.safe_at k h i).vote2BeforeTimeout i v2 ⟨(N.trace k h i), (Trace.history_getElem? _ (Nat.lt_succ_self i)), hi⟩ he2 tv (by
        rw [Trace.history_upTo _ (Nat.le_refl _)]
        exact (Trace.sent_history _).mpr ⟨j, Nat.lt_succ_of_le hji, hj⟩) het
    exact absurd hle (Nat.not_le_of_lt hlt)
  · have hcover := (N.safe_at k h j).timeoutLock j tv ⟨(N.trace k h j), (Trace.history_getElem? _ (Nat.lt_succ_self j)), hj⟩ het v2
    rw [Trace.history_upTo _ (Nat.le_succ j)] at hcover
    exact hcover ((Trace.sent_history _).mpr ⟨i, by omega, hi⟩) he2

/-- A `Cert1` an honest node holds, other than the anchor's, it received at some step. -/
theorem cert1_received {k : PubKey} {h : C.Honest k} {n : Nat} {c : Cert1}
    (hc : ((N.trace k h).history n).HasCert1 cfg c) (hne : c ≠ cfg.anchorCert) :
    ∃ m, (N.trace k h m).input = .certificate1 c ∨ ∃ c2 p, (N.trace k h m).input = .epochChange c c2 p := by
  rcases hc with hanc | hrec | ⟨c2, p, hrec⟩
  · exact absurd hanc hne
  · obtain ⟨m, -, hm⟩ := (Trace.received_history _).mp hrec
    exact ⟨m, Or.inl hm⟩
  · obtain ⟨m, -, hm⟩ := (Trace.received_history _).mp hrec
    exact ⟨m, Or.inr ⟨c2, p, hm⟩⟩

/-- A `Cert1` an honest node holds is backed, or is the anchor's. -/
theorem cert1_held_backed {k : PubKey} {h : C.Honest k} {n : Nat} {c : Cert1}
    (hc : ((N.trace k h).history n).HasCert1 cfg c) :
    c = cfg.anchorCert ∨ Cert1Backed N.trace c := by
  by_cases hne : c = cfg.anchorCert
  · exact Or.inl hne
  · obtain ⟨m, hm⟩ := cert1_received N hc hne
    refine N.cert1Genuine k h m c ?_
    rcases hm with hm | ⟨c2, p, hm⟩ <;> rw [hm] <;> rfl

end NewProtocol
