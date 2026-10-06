module

public import NewProtocolSpec.Network.Defs
public import NewProtocolSpec.Network.Lemmas

/-!
# What certificates guarantee

The properties of certificates the safety argument uses, proved from the
committee and the runs of `NewProtocolSpec.Network.Defs`.
-/

@[expose] public section

namespace NewProtocol

variable (cfg : Config)

/--
A quorum's timeout for view `x` means an honest node was in view `x`.

The fact the temporal half of the safety argument runs on. A quorum contains
an honest member, and that member's timeout vote was either its own timer —
which by `timeoutVoteSound` names the view it is in — or a vote joined on the
one-honest threshold, which by `oneHonest_reached` only happens when an honest
node really was there.

This is why a proposal that skips a view cannot exist before that view has been
left behind, and hence why it cannot be admitted by a node that has already
locked past it.
-/
theorem timeoutCert_reached {C : Committee} (N : Network cfg C) {tc : TimeoutCert}
    (h : TimeoutCertBacked N.run tc) :
    ∃ j, ∃ hj : C.honest j, ∃ m, (Run.state (N.run j hj) m).currentView = tc.view := by
  obtain ⟨q, hq, hcast⟩ := h
  obtain ⟨k, hk, -, hh⟩ := C.intersect q q hq hq
  obtain ⟨n, o, hmem, e, rfl⟩ := hcast k hk hh
  rcases timeoutVote_view (N.run k hh) hmem with hcur | hone
  · exact ⟨k, hh, n, hcur⟩
  · exact oneHonest_reached cfg N k hh n tc.view hone

/--
**At most one block is `Cert1`-certified per view.**

Two quorums share an honest member, and an honest node votes once per view,
so the two certificates carry the same block. Once an assumption,
no longer assumed.
-/
theorem cert1_unique {C : Committee} (N : Network cfg C) {c c' : Cert1}
    (h : Network.ValidCert1 cfg N c) (h' : Network.ValidCert1 cfg N c')
    (hv : c.view = c'.view) : c.data.blockHash = c'.data.blockHash := by
  obtain ⟨k, hh, n, m, h1, h2⟩ := valid1_shared cfg h h'
  exact vote1_agree h1 h2 hv

/--
**A `Cert2` presupposes the matching `Cert1`.**

Once an assumption, now a theorem. An honest member of the
quorum cast vote2, and `Vote2Justification.certMatches` says it did so only
holding a `Cert1` over exactly the block it voted for; `cert1_backed`
turns what that node holds into what the network signed.

The genesis side condition is discharged, not assumed: the vote required an
admitted proposal, and nothing is ever admitted at genesis because admission
demands a view above the bar.
-/
theorem cert2_implies_cert1 {C : Committee} (N : Network cfg C)
    (hcfg : ConfigCoherent cfg) {c2 : Cert2}
    (h : Network.ValidCert2 cfg N c2) :
    ∃ c1 : Cert1, Cert1Backed N.run c1 ∧ c1.view = c2.view
      ∧ c1.data.blockHash = c2.data.blockHash := by
  obtain ⟨q, hq, hcast⟩ := h
  obtain ⟨k, hk, -, hh⟩ := C.intersect q q hq hq
  obtain ⟨n, o, hmem, rfl⟩ := hcast k hk hh
  obtain ⟨c1, hheld, hkey, hhash, hne⟩ :=
    vote2_holds_cert1 (N.run k hh) (N.start k hh) hcfg hmem
  exact ⟨c1, cert1_backed cfg N hcfg k hh (n + 1) c2.view c1
    (fun hz => hne (hkey ▸ hz)) hheld, hkey, hhash⟩

/--
**At most one block is `Cert2`-certified per view.**

Two quorums share an honest member, and an honest node votes once per view,
so the two certificates carry the same block. Once an assumption,
no longer assumed.
-/
theorem cert2_unique {C : Committee} (N : Network cfg C) {c c' : Cert2}
    (h : Network.ValidCert2 cfg N c) (h' : Network.ValidCert2 cfg N c')
    (hv : c.view = c'.view) : c.data.blockHash = c'.data.blockHash := by
  obtain ⟨k, hh, n, m, h1, h2⟩ := valid2_shared cfg h h'
  exact vote2_agree h1 h2 hv

end NewProtocol
