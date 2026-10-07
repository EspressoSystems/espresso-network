module

public import NewProtocolImpl.Conformance.Safety

/-!
# Nothing owed, before a step and after a collection

Two of the three `Settled` obligations: the fresh node (`InitialSettled`) and
the collected one (`GcSettles`). The third, that a step leaves nothing owed, is
`NewProtocolImpl.Conformance.Owed`.

Collection is the interesting one, and it is proved *entirely from `GcSpec`* —
the machine's filters never appear, only the rule they were shown to satisfy.
Each of the four enabledness predicates is carried backwards across a
collection: if an action is owed afterwards it was owed before, so a state that
owed nothing still owes nothing. Everything an action reads is retained where the
action could still be taken, and the marks that make it stale are retained there
too; the one predicate that reads a *set* rather than a slot is
`NodeState.aboveDecideFloor`, and `GcSpec.floorStable` carries it back.
-/

@[expose] public section

set_option linter.unusedSectionVars false

open Std (TreeMap TreeSet)

namespace NewProtocol
namespace Impl

variable {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
variable {a a' : NodeState}

/-! ## The floor

The one fact about the floor the transfers below need, and it is not about
collection: being above the floor is an upward-closed condition on views.
-/

/-- A view above the floor stays above it as the view rises. -/
theorem aboveDecideFloor_mono {v w : ViewNumber} (h : a.aboveDecideFloor cfg v)
    (hle : v ≤ w) : a.aboveDecideFloor cfg w :=
  fun u hu => Nat.lt_of_lt_of_le (h u hu) hle

/-! ## What survives a collection

One lemma per action, each read as: an action owed after the collection was owed
before it.
-/

/-- A vote1 owed after a collection was owed before it. -/
theorem vote1Enabled_of_gc (hgc : GcSpec cfg a a') {p : Proposal}
    (hen : Vote1Enabled a' p) : Vote1Enabled a p := by
  obtain ⟨hj, hver, hfresh, htimeout, hbar, hlock⟩ := hen
  obtain ⟨hadm, hvalid, hvid, hsafe, hparent⟩ := hj
  refine ⟨⟨hgc.shrinks.admitted _ _ hadm, hvalid, ?_, hgc.lockSame ▸ hsafe, ?_⟩,
    hgc.shrinks.validated _ _ hver,
    fun hv => hfresh (hgc.voted1Retained _ hbar hv), hgc.timeoutViewSame ▸ htimeout,
    Nat.lt_of_le_of_lt hgc.barredViewMono hbar, hgc.lockSame ▸ hlock⟩
  · obtain ⟨sh, hsh⟩ := Option.isSome_iff_exists.mp hvid
    exact Option.isSome_iff_exists.mpr ⟨sh, hgc.shrinks.vidShares _ _ hsh⟩
  · intro hne
    obtain ⟨parent, hp, hh, hrec⟩ := hparent hne
    exact ⟨parent, hgc.shrinks.proposals _ _ hp, hh,
      hgc.shrinks.blocksReconstructed _ _ hrec⟩

/--
A vote2 owed after a collection was owed before it.

The branch record is the delicate one: it bars a vote2, so the *absence*
of a record afterwards has to mean absence before, which is the only place
`GcSpec.vote1BranchesRetained` is used. Its watermark is the floor rather than
the bar precisely so that this holds — and the record that would bar the vote
sits above the view voted in, hence above the floor too.
-/
theorem vote2Enabled_of_gc (hgc : GcSpec cfg a a') {p : Proposal}
    (hen : Vote2Enabled cfg a' p) : Vote2Enabled cfg a p := by
  obtain ⟨hj, haround, hfresh, hcert2, hdec, habove, hbar⟩ := hen
  obtain ⟨hadm, ⟨c1, hc1, hc1h⟩, hrec⟩ := hj
  have habove' : a.aboveDecideFloor cfg p.viewNumber := hgc.floorStable _ habove
  refine ⟨⟨hgc.shrinks.admitted _ _ hadm, ⟨c1, hgc.shrinks.cert1s _ _ hc1, hc1h⟩,
      hgc.shrinks.blocksReconstructed _ _ hrec⟩, ?_,
    fun hv => hfresh (hgc.voted2Retained _ habove' hv), ?_,
    fun hd => hdec (hgc.decidedRetained _ habove' hd), habove',
    Nat.lt_of_le_of_lt hgc.barredViewMono hbar⟩
  · intro ⟨w, u, hlt, hbr, hu⟩
    exact haround ⟨w, u, hlt, hgc.vote1BranchesRetained _ _
      (aboveDecideFloor_mono habove' (Nat.le_of_lt hlt)) hbr, hu⟩
  · rcases hc2 : a.cert2s p.viewNumber with _ | c2
    · rfl
    · exact absurd ((hgc.keepsDecideAboveFloor _ habove').cert2s c2 hc2) (by rw [hcert2]; simp)

/-- A decide owed after a collection was owed before it. -/
theorem decideEnabled_of_gc (hgc : GcSpec cfg a a') {v : ViewNumber}
    (hen : DecideEnabled cfg a' v) : DecideEnabled cfg a v := by
  obtain ⟨hdec, habove, hc1, c2, p, hc2, hp, hh⟩ := hen
  have habove' : a.aboveDecideFloor cfg v := hgc.floorStable _ habove
  refine ⟨fun hd => hdec (hgc.decidedRetained _ habove' hd), habove', ?_,
    c2, p, hgc.shrinks.cert2s _ _ hc2, hgc.shrinks.proposals _ _ hp, hh⟩
  obtain ⟨c1, hc1'⟩ := Option.isSome_iff_exists.mp hc1
  exact Option.isSome_iff_exists.mpr ⟨c1, hgc.shrinks.cert1s _ _ hc1'⟩

/-- A proposal owed after a collection was owed before it. -/
theorem proposeEnabled_of_gc (hgc : GcSpec cfg a a') {p : Proposal}
    (hen : ProposeEnabled leader node a' p) : ProposeEnabled leader node a p := by
  obtain ⟨hj, hfresh, htimeout, hbar⟩ := hen
  obtain ⟨hlead, hwf, hjust, parent, hp, hh, hhdr⟩ := hj
  refine ⟨⟨hlead, hwf, ?_, parent, hgc.shrinks.proposals _ _ hp, hh,
      hgc.shrinks.headers _ _ _ hhdr⟩,
    fun hv => hfresh (hgc.proposedRetained _ hbar hv), hgc.timeoutViewSame ▸ htimeout,
    Nat.lt_of_le_of_lt hgc.barredViewMono hbar⟩
  revert hjust
  unfold ParentCertJustified
  cases p.timeoutEvidence with
  | some tc => exact fun ⟨htc, hlk⟩ => ⟨hgc.shrinks.timeoutCerts _ _ htc, hgc.lockSame ▸ hlk⟩
  | none => exact fun ⟨hc1, hv⟩ => ⟨hgc.shrinks.cert1s _ _ hc1, hv⟩

/-- The machine meets `GcSettles`. -/
theorem gc_settles (cfg : Config) (leader : ViewNumber → Option PubKey) (node : PubKey) :
    GcSettles cfg leader node := by
  intro s hwf hs
  obtain ⟨h1, h2, hd, hp⟩ := hs
  have hgc := gc_conforms cfg s hwf
  exact
    { vote1 := fun p hen => h1 p (vote1Enabled_of_gc hgc hen)
      vote2 := fun p hen => h2 p (vote2Enabled_of_gc hgc hen)
      decide := fun v hen => hd v (decideEnabled_of_gc hgc hen)
      propose := fun p hen => hp p (proposeEnabled_of_gc hgc hen) }

/-! ## What a fresh node owes

Nothing, and for a blunter reason than above: a fresh node has admitted nothing,
holds no `Cert2` and has no built header, so three of the four
justifications fail outright and the fourth has no block to propose.
-/

/-- The machine meets `InitialSettled`. -/
theorem initial_settled (cfg : Config) (leader : ViewNumber → Option PubKey) (node : PubKey) :
    InitialSettled cfg leader node := by
  intro _
  have habs : (initial cfg).abstract = NodeState.initial cfg := initial_abstract cfg
  refine { vote1 := ?_, vote2 := ?_, decide := ?_, propose := ?_ } <;> rw [habs]
  · exact fun p hen => absurd hen.1.proposalAdmitted (by simp [NodeState.initial])
  · exact fun p hen => absurd hen.1.proposalAdmitted (by simp [NodeState.initial])
  · intro v hen
    obtain ⟨_, _, _, c2, p, hc2, _, _⟩ := hen
    exact absurd hc2 (by simp [NodeState.initial])
  · intro p hen
    obtain ⟨_, _, _, parent, _, _, hhdr⟩ := hen.1
    exact absurd hhdr (by simp [NodeState.initial])

end Impl
end NewProtocol
