module

public import NewProtocolSpec.Proofs.Traces
public import NewProtocolSpec.Properties

/-!
# A network the safety premises hold in

A premise nothing satisfies makes a result true for no reason, and no build
notices. So here is a network meeting every premise of `NoFork` in which a
block really is certified and committed, and a proposal really opens a new
epoch behind that commit, so that the rule for opening an epoch
(`OpensEpochJustified`) is met by doing something rather than by having nothing to
do.

Honesty changes with the epoch. `me` is the committee of epoch one and honest
in it; `you` is the committee of every later epoch and honest in those. A
faulty node `other` proposes. One block to an epoch, so the second block opens
epoch two.

`me` receives the first block, votes on it, receives its certificates, decides
it, then receives the second block and, being faulty in epoch two, votes1 for it
and for another block in the same view. `you` receives the first block and its
`Cert2`, then the second block, and votes1 on it behind that `Cert2`
(`OpensEpochJustified`); its vote is the `Cert1` of epoch two. It then votes2,
its vote is the `Cert2` of epoch two, and it decides the second block. So two
blocks of different epochs are committed, each node decides one, and the network
meets the premises with a node that breaks the signing rules in an epoch it is not
honest in (`Witness.per_epoch`).

`CollisionFree` and `BlockValid` are taken as hypotheses: both are opaque, the
first a cryptographic assumption and the second the application's. They are
consistent together: `BlockValid` holding of every block, and a hash that is
injective on blocks, are a model of both.
-/

@[expose] public section

namespace NewProtocol
namespace Witness

/-- Honest up to epoch one: the committee of epochs zero and one. -/
def me : PubKey := ⟨1⟩

/-- Honest from epoch two on. -/
def you : PubKey := ⟨3⟩

/-- The faulty proposer. -/
def other : PubKey := ⟨2⟩

/-- Genesis: block zero, in epoch one. -/
def anchorB : Block :=
  ⟨⟨⟨0⟩, 0⟩, ViewNumber.genesis, epochOf 0 1, ⟨⟨⟨0⟩, ⟨0⟩, 0⟩, ViewNumber.genesis⟩, none, ⟨7⟩⟩

/-- One block to an epoch. -/
def cfg : Config where
  anchorBlock := anchorB
  anchorCert := ⟨⟨blockHash anchorB, epochOf 0 1, 0⟩, ViewNumber.genesis⟩
  decideBuffer := 20
  epochHeight := 1

theorem cfg_coherent : ConfigCoherent cfg where
  anchorCertView := rfl
  anchorBlockEpoch := rfl
  anchorCertBlock := rfl
  anchorCertBlockNumber := rfl
  anchorCertEpoch := rfl

/-- The first block, the last of epoch one. -/
def blk1 : Block := ⟨⟨⟨3⟩, 1⟩, ⟨1⟩, epochOf 1 1, cfg.anchorCert, none, ⟨9⟩⟩

/-- The certificates over it. -/
def cert1₁ : Cert1 := ⟨⟨blockHash blk1, blk1.epoch, 1⟩, ⟨1⟩⟩

def cert2₁ : Cert2 := ⟨cert1₁.data.toVote2, ⟨1⟩⟩

/-- The second block, the first of epoch two. -/
def blk2 : Block := ⟨⟨⟨5⟩, 2⟩, ⟨2⟩, epochOf 2 1, cert1₁, none, ⟨10⟩⟩

/-- Another block for the second block's view, which `me` also votes for. -/
def blk2' : Block := ⟨⟨⟨5⟩, 2⟩, ⟨2⟩, epochOf 2 1, cert1₁, none, ⟨11⟩⟩

/-- The certificate `you`'s vote forms. -/
def cert1₂ : Cert1 := ⟨⟨blockHash blk2, blk2.epoch, 2⟩, ⟨2⟩⟩

/-- The `Cert2` `you`'s vote2 forms, the commit of epoch two. -/
def cert2₂ : Cert2 := ⟨cert1₂.data.toVote2, ⟨2⟩⟩

def share1 : VidShare := ⟨⟨1⟩, ⟨3⟩⟩

def share2 : VidShare := ⟨⟨2⟩, ⟨5⟩⟩

/-- `me`'s votes. -/
def vote1₁ : Vote1 := ⟨cert1₁.data, cert1₁.view, me⟩

def vote2₁ : Vote2 := ⟨cert2₁.data, cert2₁.view, me⟩

def vote1₂ : Vote1 := ⟨cert1₂.data, cert1₂.view, me⟩

/-- `me`'s second vote1 in the second block's view, for the other block. -/
def bad : Vote1 := ⟨⟨blockHash blk2', blk2'.epoch, 2⟩, ⟨2⟩, me⟩

/-- `you`'s vote1 on the second block. -/
def voteYou : Vote1 := ⟨cert1₂.data, cert1₂.view, you⟩

/-- `you`'s vote2 on the second block. -/
def vote2You : Vote2 := ⟨cert2₂.data, cert2₂.view, you⟩

/-- `me`'s trace. After the scripted steps it only receives inputs that ask nothing of it. -/
def script : Trace
  | 0 => ⟨.proposal other blk1 (some share1), []⟩
  | 1 => ⟨.blockValidated ⟨1⟩ (blockHash blk1), [.send (.vote1 vote1₁)]⟩
  | 2 => ⟨.certificate1 cert1₁, []⟩
  | 3 => ⟨.blockReconstructed ⟨1⟩ blk1.payloadCommit, [.send (.vote2 vote2₁)]⟩
  | 4 => ⟨.certificate2 cert2₁, [.decided [blk1] cert1₁ cert2₁]⟩
  | 5 => ⟨.proposal other blk2 (some share2), []⟩
  | 6 => ⟨.blockValidated ⟨2⟩ (blockHash blk2), [.send (.vote1 vote1₂), .send (.vote1 bad)]⟩
  | _ + 7 => ⟨.blockValidated ⟨0⟩ ⟨0⟩, []⟩

/-- `you`'s trace. -/
def yours : Trace
  | 0 => ⟨.proposal other blk1 none, []⟩
  | 1 => ⟨.certificate2 cert2₁, []⟩
  | 2 => ⟨.proposal other blk2 (some share2), []⟩
  | 3 => ⟨.blockValidated ⟨2⟩ (blockHash blk2), [.send (.vote1 voteYou)]⟩
  | 4 => ⟨.certificate1 cert1₂, []⟩
  | 5 => ⟨.blockReconstructed ⟨2⟩ blk2.payloadCommit, [.send (.vote2 vote2You)]⟩
  | 6 => ⟨.certificate2 cert2₂, [.decided [blk2] cert1₂ cert2₂]⟩
  | _ + 7 => ⟨.blockValidated ⟨0⟩ ⟨0⟩, []⟩

/-- The committee of an epoch, alone in it: `me` up to epoch one, `you` after. -/
def lead (e : EpochNumber) : PubKey := if e.toNat ≤ 1 then me else you

/-- Each epoch's committee is its `lead`, honest in that epoch. -/
def com : Committee where
  honest e k := k = lead e
  members e k := k = lead e
  Quorum e q := q (lead e)
  intersect e _ _ hq hq' := ⟨lead e, hq, hq', rfl⟩
  honestFinite := ⟨[me, you], fun e _ hk => by subst hk; unfold lead; split <;> simp⟩

/-- The honest members of every epoch's committee are a quorum. -/
theorem members_quorum : ∀ e, com.Quorum e fun k => com.members e k ∧ com.honest e k := fun _ =>
  ⟨rfl, rfl⟩

/-- Each node's trace. -/
def tr (k : PubKey) : Trace := if k = me then script else yours

theorem tr_me : tr me = script := ite_eq_left rfl

theorem tr_you : tr you = yours := ite_eq_right (by decide)

/-- The nodes with a trace are `me` and `you`. -/
theorem honest_cases {k : PubKey} (h : com.Honest k) : k = me ∨ k = you := by
  obtain ⟨e, rfl⟩ := h
  show lead e = me ∨ lead e = you
  unfold lead; split
  · exact Or.inl rfl
  · exact Or.inr rfl

/-! ## What the traces send and receive -/

theorem getElem?_history {r : Trace} {n i : Nat} {st : Step} (h : (r.history n)[i]? = some st) :
    i < n ∧ r i = st := by
  simp only [Trace.history, List.getElem?_map] at h
  by_cases hi : i < n
  · rw [List.getElem?_range hi] at h
    exact ⟨hi, Option.some.inj h⟩
  · rw [List.getElem?_eq_none (by simpa using hi)] at h
    cases h

/-- The only proposals `me` receives are the two blocks. -/
theorem proposals_received {i : Nat} {sender : PubKey} {p : Proposal} {share : Option VidShare}
    (h : (script i).input = .proposal sender p share) : p = blk1 ∨ p = blk2 := by
  match i, h with
  | 0, h => cases h; exact Or.inl rfl
  | 5, h => cases h; exact Or.inr rfl
  | 1, h | 2, h | 3, h | 4, h | 6, h | _ + 7, h => cases h

/-- The only vote1s `me` sends are the three the script names. -/
theorem vote1s_sent {i : Nat} {v : Vote1} (h : Output.send (.vote1 v) ∈ (script i).output) :
    v = vote1₁ ∨ v = vote1₂ ∨ v = bad := by
  match i, h with
  | 1, h => simp [script] at h; exact Or.inl h
  | 6, h =>
    simp [script] at h
    rcases h with h | h
    · exact Or.inr (Or.inl h)
    · exact Or.inr (Or.inr h)
  | 0, h | 2, h | 3, h | 5, h | _ + 7, h => simp [script] at h
  | 4, h => simp [script] at h

/-- The only vote2 `me` sends is the first block's. -/
theorem vote2s_sent {i : Nat} {v : Vote2} (h : Output.send (.vote2 v) ∈ (script i).output) :
    v = vote2₁ := by
  match i, h with
  | 3, h => simp [script] at h; exact h
  | 0, h | 1, h | 2, h | 5, h | 6, h | _ + 7, h => simp [script] at h
  | 4, h => simp [script] at h

/-- `me` sends no timeout vote. -/
theorem no_timeoutVotes {i : Nat} {v : TimeoutVote} :
    Output.send (.timeoutVote v) ∉ (script i).output := by
  intro h
  match i, h with
  | 0, h | 1, h | 2, h | 3, h | 5, h | 6, h | _ + 7, h => simp [script] at h
  | 4, h => simp [script] at h

/-- The outputs of `you`: its two votes on the second block, and its decide of it. -/
theorem yours_output {i : Nat} {o : Output} (h : o ∈ (yours i).output) :
    (i = 3 ∧ o = .send (.vote1 voteYou)) ∨ (i = 5 ∧ o = .send (.vote2 vote2You))
      ∨ (i = 6 ∧ o = .decided [blk2] cert1₂ cert2₂) := by
  match i, h with
  | 3, h => simp [yours] at h; exact Or.inl ⟨rfl, h⟩
  | 5, h => simp [yours] at h; exact Or.inr (Or.inl ⟨rfl, h⟩)
  | 6, h => simp [yours] at h; exact Or.inr (Or.inr ⟨rfl, h⟩)
  | 0, h | 1, h | 2, h | 4, h | _ + 7, h => simp [yours] at h

/-! ## Each node obeys the signing rules in the epochs it is honest in -/

/-- `me` is honest in an epoch only if it is epoch one or earlier. -/
theorem me_honest {e : EpochNumber} (h : com.honest e me) : e.toNat ≤ 1 := by
  show e.toNat ≤ 1
  have h' : me = lead e := h
  unfold lead at h'; split at h'
  · assumption
  · exact absurd h' (by decide)

/-- `me` is not honest in the epoch of the second block. -/
theorem me_faulty2 : ¬ com.honest blk2.epoch me := fun h => absurd (me_honest h) (by decide)

theorem me_faulty2' : ¬ com.honest blk2'.epoch me := fun h => absurd (me_honest h) (by decide)

theorem safeMe (n : Nat) (hv : ∀ b, BlockValid b) :
    SafeHistory cfg me (com.honest · me) (script.history n) where
  vote1Justified := fun i vote ⟨st, hst, hmem⟩ hg => by
    obtain ⟨hi, rfl⟩ := getElem?_history hst
    rw [Trace.history_upTo _ (Nat.succ_le_of_lt hi)]
    match i, hmem with
    | 1, hmem =>
      simp [script] at hmem; subst hmem
      exact ⟨rfl, Or.inl ⟨other, blk1, share1, (Trace.received_history _).mpr ⟨0, by omega, rfl⟩,
        ⟨by decide, Or.inl ⟨rfl, rfl⟩, rfl, rfl⟩, hv _, fun _ h => by simp [blk1] at h,
        fun hent => absurd hent.1 (by decide), rfl, rfl⟩⟩
    | 6, hmem =>
      simp [script] at hmem
      rcases hmem with rfl | rfl
      · exact absurd hg me_faulty2
      · exact absurd hg me_faulty2'
    | 0, hmem | 2, hmem | 3, hmem | 5, hmem | _ + 7, hmem => simp [script] at hmem
    | 4, hmem => simp [script] at hmem
  vote1Once v v' hs hs' hg heq hview := by
    obtain ⟨i, -, hi⟩ := (Trace.sent_history _).mp hs
    obtain ⟨j, -, hj⟩ := (Trace.sent_history _).mp hs'
    rcases vote1s_sent hi with rfl | rfl | rfl
    · rcases vote1s_sent hj with rfl | rfl | rfl
      · rfl
      · exact absurd heq (by decide)
      · exact absurd heq (by decide)
    · exact absurd hg me_faulty2
    · exact absurd hg me_faulty2'
  vote2Justified := fun i vote ⟨st, hst, hmem⟩ _ => by
    obtain ⟨hi, rfl⟩ := getElem?_history hst
    rw [Trace.history_upTo _ (Nat.succ_le_of_lt hi)]
    match i, hmem with
    | 3, hmem =>
      simp [script] at hmem; subst hmem
      refine ⟨rfl, by decide, cert1₁, blk1, ?_, ?_, ⟨Nat.le_refl _, rfl⟩, ?_, rfl, rfl⟩
      · exact Or.inr (Or.inl ((Trace.received_history _).mpr ⟨2, by omega, rfl⟩))
      · exact Or.inr (Or.inl ⟨other, share1, (Trace.received_history _).mpr ⟨0, by omega, rfl⟩⟩)
      · exact Or.inr ((Trace.received_history _).mpr ⟨3, by omega, rfl⟩)
    | 0, hmem | 1, hmem | 2, hmem | 5, hmem | 6, hmem | _ + 7, hmem => simp [script] at hmem
    | 4, hmem => simp [script] at hmem
  vote2Once v v' hs hs' _ _ _ := by
    obtain ⟨i, -, hi⟩ := (Trace.sent_history _).mp hs
    obtain ⟨j, -, hj⟩ := (Trace.sent_history _).mp hs'
    rw [vote2s_sent hi, vote2s_sent hj]
  vote2BeforeTimeout := fun i vote ⟨st, hst, _⟩ _ tv hs => by
    obtain ⟨hi, -⟩ := getElem?_history hst
    rw [Trace.history_upTo _ (Nat.succ_le_of_lt hi)] at hs
    obtain ⟨j, -, hj⟩ := (Trace.sent_history _).mp hs
    exact absurd hj no_timeoutVotes
  timeoutLock := fun i vote ⟨st, hst, hmem⟩ => by
    obtain ⟨-, rfl⟩ := getElem?_history hst
    exact absurd hmem no_timeoutVotes
  decideJustified i st blocks c1 c2 hst hmem _ := by
    obtain ⟨hi, rfl⟩ := getElem?_history hst
    rw [Trace.history_upTo _ (Nat.succ_le_of_lt hi)]
    match i, hmem with
    | 4, hmem =>
      simp [script] at hmem
      obtain ⟨rfl, rfl, rfl⟩ := hmem
      refine ⟨blk1, [], rfl, Or.inl ((Trace.received_history _).mpr ⟨4, by omega, rfl⟩),
        ⟨Nat.le_refl _, rfl⟩, Or.inr (Or.inl ((Trace.received_history _).mpr ⟨2, by omega, rfl⟩)),
        ⟨Nat.le_refl _, rfl⟩, trivial, ?_⟩
      intro b hb
      simp at hb; subst hb
      exact ⟨Or.inr (Or.inl ⟨other, share1, (Trace.received_history _).mpr ⟨0, by omega, rfl⟩⟩),
        by decide⟩
    | 0, hmem | 1, hmem | 2, hmem | 3, hmem | 5, hmem | 6, hmem | _ + 7, hmem =>
      simp [script] at hmem
  decideOnce i st j blocks c1 c2 hst hj _ := by
    obtain ⟨hi, rfl⟩ := getElem?_history hst
    have hmem := List.mem_of_getElem? hj
    match i, hmem with
    | 4, hmem =>
      simp [script] at hmem
      obtain ⟨rfl, rfl, rfl⟩ := hmem
      refine ⟨by simp, fun b hb => ?_⟩
      simp at hb; subst hb
      have hj0 : j = 0 := by
        cases j with
        | zero => rfl
        | succ j => simp [script] at hj
      subst hj0
      rintro ⟨st', hst', blocks', c1', c2', b', hdec, -, -⟩
      rw [Trace.history_upTo _ (by omega)] at hst'
      rcases List.mem_append.mp hst' with hst' | hst'
      · obtain ⟨m, hm, rfl⟩ := (Trace.mem_history _).mp hst'
        match m, hm, hdec with
        | 0, _, hdec | 1, _, hdec | 2, _, hdec | 3, _, hdec => simp [script] at hdec
      · simp at hst'; subst hst'; simp at hdec
    | 0, hmem | 1, hmem | 2, hmem | 3, hmem | 5, hmem | 6, hmem | _ + 7, hmem =>
      simp [script] at hmem

/-- `you` sends no timeout vote. -/
theorem you_no_timeoutVotes {i : Nat} {v : TimeoutVote} :
    Output.send (.timeoutVote v) ∉ (yours i).output := fun h => by
  rcases yours_output h with ⟨-, h⟩ | ⟨-, h⟩ | ⟨-, h⟩ <;> cases h

/-- `you` obeys the signing rules in every epoch. -/
theorem safeYou (P : EpochNumber → Prop) (n : Nat) (hv : ∀ b, BlockValid b) :
    SafeHistory cfg you P (yours.history n) where
  vote1Justified := fun i vote ⟨st, hst, hmem⟩ _ => by
    obtain ⟨hi, rfl⟩ := getElem?_history hst
    rcases yours_output hmem with ⟨rfl, ho⟩ | ⟨-, ho⟩ | ⟨-, ho⟩ <;> cases ho
    rw [Trace.history_upTo _ (Nat.succ_le_of_lt hi)]
    exact ⟨rfl, Or.inl ⟨other, blk2, share2, (Trace.received_history _).mpr ⟨2, by omega, rfl⟩,
      ⟨by decide, Or.inl ⟨rfl, rfl⟩, rfl, rfl⟩, hv _, fun _ h => by simp [blk2] at h,
      fun _ => ⟨⟨blk1, Or.inr (Or.inl ⟨other, none, (Trace.received_history _).mpr ⟨0, by omega, rfl⟩⟩),
        rfl, rfl⟩, cert2₁, Or.inl (Or.inl ((Trace.received_history _).mpr ⟨1, by omega, rfl⟩)), by decide, rfl⟩,
      rfl, rfl⟩⟩
  vote1Once v v' hs hs' _ _ _ := by
    obtain ⟨i, -, hi⟩ := (Trace.sent_history _).mp hs
    obtain ⟨j, -, hj⟩ := (Trace.sent_history _).mp hs'
    rcases yours_output hi with ⟨-, hi⟩ | ⟨-, hi⟩ | ⟨-, hi⟩ <;> cases hi
    rcases yours_output hj with ⟨-, hj⟩ | ⟨-, hj⟩ | ⟨-, hj⟩ <;> cases hj
    rfl
  vote2Justified := fun i vote ⟨st, hst, hmem⟩ _ => by
    obtain ⟨hi, rfl⟩ := getElem?_history hst
    rcases yours_output hmem with ⟨-, ho⟩ | ⟨rfl, ho⟩ | ⟨-, ho⟩ <;> cases ho
    rw [Trace.history_upTo _ (Nat.succ_le_of_lt hi)]
    refine ⟨rfl, by decide, cert1₂, blk2, ?_, ?_, ⟨Nat.le_refl _, rfl⟩, ?_, rfl, rfl⟩
    · exact Or.inr (Or.inl ((Trace.received_history _).mpr ⟨4, by omega, rfl⟩))
    · exact Or.inr (Or.inl ⟨other, some share2, (Trace.received_history _).mpr ⟨2, by omega, rfl⟩⟩)
    · exact Or.inr ((Trace.received_history _).mpr ⟨5, by omega, rfl⟩)
  vote2Once v v' hs hs' _ _ _ := by
    obtain ⟨i, -, hi⟩ := (Trace.sent_history _).mp hs
    obtain ⟨j, -, hj⟩ := (Trace.sent_history _).mp hs'
    rcases yours_output hi with ⟨-, hi⟩ | ⟨-, hi⟩ | ⟨-, hi⟩ <;> cases hi
    rcases yours_output hj with ⟨-, hj⟩ | ⟨-, hj⟩ | ⟨-, hj⟩ <;> cases hj
    rfl
  vote2BeforeTimeout := fun i vote ⟨st, hst, _⟩ _ tv hs => by
    obtain ⟨hi, -⟩ := getElem?_history hst
    rw [Trace.history_upTo _ (Nat.succ_le_of_lt hi)] at hs
    obtain ⟨j, -, hj⟩ := (Trace.sent_history _).mp hs
    exact absurd hj you_no_timeoutVotes
  timeoutLock := fun i vote ⟨st, hst, hmem⟩ => by
    obtain ⟨-, rfl⟩ := getElem?_history hst
    exact absurd hmem you_no_timeoutVotes
  decideJustified i st blocks c1 c2 hst hmem _ := by
    obtain ⟨hi, rfl⟩ := getElem?_history hst
    rcases yours_output hmem with ⟨-, ho⟩ | ⟨-, ho⟩ | ⟨rfl, ho⟩ <;> cases ho
    rw [Trace.history_upTo _ (Nat.succ_le_of_lt hi)]
    refine ⟨blk2, [], rfl, Or.inl ((Trace.received_history _).mpr ⟨6, by omega, rfl⟩),
      ⟨Nat.le_refl _, rfl⟩, Or.inr (Or.inl ((Trace.received_history _).mpr ⟨4, by omega, rfl⟩)),
      ⟨Nat.le_refl _, rfl⟩, trivial, ?_⟩
    intro b hb
    simp at hb; subst hb
    exact ⟨Or.inr (Or.inl ⟨other, some share2, (Trace.received_history _).mpr ⟨2, by omega, rfl⟩⟩),
      by decide⟩
  decideOnce i st j blocks c1 c2 hst hj _ := by
    obtain ⟨hi, rfl⟩ := getElem?_history hst
    rcases yours_output (List.mem_of_getElem? hj) with ⟨-, ho⟩ | ⟨-, ho⟩ | ⟨rfl, ho⟩ <;> cases ho
    refine ⟨by simp, fun b hb => ?_⟩
    simp at hb; subst hb
    have hj0 : j = 0 := by
      cases j with
      | zero => rfl
      | succ j => simp [yours] at hj
    subst hj0
    rintro ⟨st', hst', blocks', c1', c2', b', hdec, -, -⟩
    rw [Trace.history_upTo _ (by omega)] at hst'
    rcases List.mem_append.mp hst' with hst' | hst'
    · obtain ⟨m, hm, rfl⟩ := (Trace.mem_history _).mp hst'
      rcases yours_output hdec with ⟨-, ho⟩ | ⟨-, ho⟩ | ⟨rfl, -⟩
      · cases ho
      · cases ho
      · omega
    · simp at hst'; subst hst'; simp at hdec

/-! ## The network -/

/-- The traces send the vote1s behind the two `Cert1`s. -/
theorem backed1 {c : Cert1} (hc : c = cert1₁ ∨ c = cert1₂) : Cert1Backed (C := com)
    (fun k _ => tr k) c := by
  rcases hc with rfl | rfl
  · refine ⟨fun k => k = me, show lead _ = me by decide, fun k hk _ => ?_⟩
    subst hk; show SentBy (tr me) _; rw [tr_me]; exact ⟨1, by simp [script, vote1₁]⟩
  · refine ⟨fun k => k = you, show lead _ = you by decide, fun k hk _ => ?_⟩
    subst hk; show SentBy (tr you) _; rw [tr_you]; exact ⟨3, by simp [yours, voteYou]⟩

theorem backed2 : Cert2Backed (C := com) (fun k _ => tr k) cert2₁ :=
  ⟨fun k => k = me, show lead _ = me by decide, fun k hk _ => by
    subst hk; show SentBy (tr me) _; rw [tr_me]; exact ⟨3, by simp [script, vote2₁]⟩⟩

theorem backed2' : Cert2Backed (C := com) (fun k _ => tr k) cert2₂ :=
  ⟨fun k => k = you, show lead _ = you by decide, fun k hk _ => by
    subst hk; show SentBy (tr you) _; rw [tr_you]; exact ⟨5, by simp [yours, vote2You]⟩⟩

/-- What `me` receives carries backed certificates. -/
theorem me_inputs {n : Nat} : (∀ c, c ∈ (script n).input.cert1 → c = cfg.anchorCert ∨ Cert1Backed (C := com)
      (fun k _ => tr k) c)
    ∧ (∀ c, c ∈ (script n).input.cert2 → Cert2Backed (C := com) (fun k _ => tr k) c)
    ∧ (∀ tc, tc ∉ (script n).input.timeoutCert) := by
  match n with
  | 0 => exact ⟨fun c hin => by simp [script, Input.cert1, blk1] at hin; subst hin; exact Or.inl rfl,
      fun c hin => by simp [script, Input.cert2] at hin, fun tc hin => by
        simp [script, Input.timeoutCert, blk1] at hin⟩
  | 2 => exact ⟨fun c hin => by
        simp [script, Input.cert1] at hin; subst hin; exact Or.inr (backed1 (Or.inl rfl)),
      fun c hin => by simp [script, Input.cert2] at hin, fun tc hin => by
        simp [script, Input.timeoutCert] at hin⟩
  | 4 => exact ⟨fun c hin => by simp [script, Input.cert1] at hin,
      fun c hin => by simp [script, Input.cert2] at hin; subst hin; exact backed2, fun tc hin => by
        simp [script, Input.timeoutCert] at hin⟩
  | 5 => exact ⟨fun c hin => by
        simp [script, Input.cert1, blk2] at hin; subst hin; exact Or.inr (backed1 (Or.inl rfl)),
      fun c hin => by simp [script, Input.cert2] at hin, fun tc hin => by
        simp [script, Input.timeoutCert, blk2] at hin⟩
  | 1 | 3 | 6 | _ + 7 => exact ⟨fun c hin => by simp [script, Input.cert1] at hin,
      fun c hin => by simp [script, Input.cert2] at hin, fun tc hin => by
        simp [script, Input.timeoutCert] at hin⟩

/-- What `you` receives carries backed certificates. -/
theorem you_inputs {n : Nat} : (∀ c, c ∈ (yours n).input.cert1 → c = cfg.anchorCert ∨ Cert1Backed (C := com)
      (fun k _ => tr k) c)
    ∧ (∀ c, c ∈ (yours n).input.cert2 → Cert2Backed (C := com) (fun k _ => tr k) c)
    ∧ (∀ tc, tc ∉ (yours n).input.timeoutCert) := by
  match n with
  | 1 => exact ⟨fun c hin => by simp [yours, Input.cert1] at hin,
      fun c hin => by simp [yours, Input.cert2] at hin; subst hin; exact backed2, fun tc hin => by
        simp [yours, Input.timeoutCert] at hin⟩
  | 2 => exact ⟨fun c hin => by
        simp [yours, Input.cert1, blk2] at hin; subst hin; exact Or.inr (backed1 (Or.inl rfl)),
      fun c hin => by simp [yours, Input.cert2] at hin, fun tc hin => by
        simp [yours, Input.timeoutCert, blk2] at hin⟩
  | 0 => exact ⟨fun c hin => by simp [yours, Input.cert1, blk1] at hin; subst hin; exact Or.inl rfl,
      fun c hin => by simp [yours, Input.cert2] at hin, fun tc hin => by
        simp [yours, Input.timeoutCert, blk1] at hin⟩
  | 4 => exact ⟨fun c hin => by
        simp [yours, Input.cert1] at hin; subst hin; exact Or.inr (backed1 (Or.inr rfl)),
      fun c hin => by simp [yours, Input.cert2] at hin, fun tc hin => by
        simp [yours, Input.timeoutCert] at hin⟩
  | 6 => exact ⟨fun c hin => by simp [yours, Input.cert1] at hin,
      fun c hin => by simp [yours, Input.cert2] at hin; subst hin; exact backed2', fun tc hin => by
        simp [yours, Input.timeoutCert] at hin⟩
  | 3 | 5 | _ + 7 => exact ⟨fun c hin => by simp [yours, Input.cert1] at hin,
      fun c hin => by simp [yours, Input.cert2] at hin, fun tc hin => by
        simp [yours, Input.timeoutCert] at hin⟩

/-- The only proposal `you` receives is the second block. -/
theorem you_proposals {i : Nat} {sender : PubKey} {p : Proposal} {share : Option VidShare}
    (h : (yours i).input = .proposal sender p share) : p = blk1 ∨ p = blk2 := by
  match i, h with
  | 0, h => cases h; exact Or.inl rfl
  | 2, h => cases h; exact Or.inr rfl
  | 1, h | 3, h | 4, h | 5, h | 6, h | _ + 7, h => cases h

/-- The network: `me` and `you` run their scripts. -/
def net (hv : ∀ b, BlockValid b) : Network cfg com where
  trace k _ := tr k
  safe k h n := by
    rcases honest_cases h with rfl | rfl
    · rw [tr_me]; exact safeMe n hv
    · rw [tr_you]; exact safeYou _ n hv
  cert1Genuine k h n c hin := by
    rcases honest_cases h with rfl | rfl
    · rw [tr_me] at hin; exact me_inputs.1 c hin
    · rw [tr_you] at hin; exact you_inputs.1 c hin
  cert2Genuine k h n c hin := by
    rcases honest_cases h with rfl | rfl
    · rw [tr_me] at hin; exact me_inputs.2.1 c hin
    · rw [tr_you] at hin; exact you_inputs.2.1 c hin
  timeoutCertGenuine k h n tc hin := by
    rcases honest_cases h with rfl | rfl
    · rw [tr_me] at hin; exact absurd hin (me_inputs.2.2 tc)
    · rw [tr_you] at hin; exact absurd hin (you_inputs.2.2 tc)
  revoteGenuine k h n sender r hin := by
    rcases honest_cases h with rfl | rfl
    · rw [tr_me] at hin
      match n, hin with
      | 0, hin | 1, hin | 2, hin | 3, hin | 4, hin | 5, hin | 6, hin | _ + 7, hin => cases hin
    · rw [tr_you] at hin
      match n, hin with
      | 0, hin | 1, hin | 2, hin | 3, hin | 4, hin | 5, hin | 6, hin | _ + 7, hin => cases hin

/-! ## The block tree -/

/-- The three blocks, by hash. -/
def wtree : BlockTable := fun h =>
  if h = blockHash blk2 then some blk2
  else if h = blockHash blk1 then some blk1
  else if h = blockHash anchorB then some anchorB else none

theorem wtree_coherent : TreeCoherent wtree := by
  intro h b hb
  unfold wtree at hb
  split at hb
  · cases hb; next hh => exact hh.symm
  · split at hb
    · cases hb; next hh => exact hh.symm
    · split at hb
      · cases hb; next hh => exact hh.symm
      · cases hb

/-- What `me` holds is one of the three blocks. -/
theorem me_holds {n : Nat} {b : Block} (hb : (script.history n).HasProposal cfg b) :
    b = anchorB ∨ b = blk1 ∨ b = blk2 := by
  rcases hb with rfl | ⟨sender, share, hrec⟩ | ⟨c1, c2, hrec⟩
  · exact Or.inl rfl
  · obtain ⟨m, -, hm⟩ := (Trace.received_history _).mp hrec
    rcases proposals_received hm with rfl | rfl
    · exact Or.inr (Or.inl rfl)
    · exact Or.inr (Or.inr rfl)
  · obtain ⟨m, -, hm⟩ := (Trace.received_history _).mp hrec
    match m, hm with
    | 0, hm | 1, hm | 2, hm | 3, hm | 4, hm | 5, hm | 6, hm | _ + 7, hm => cases hm

/-- What `you` holds is one of the three blocks. -/
theorem you_holds {n : Nat} {b : Block} (hb : (yours.history n).HasProposal cfg b) :
    b = anchorB ∨ b = blk1 ∨ b = blk2 := by
  rcases hb with rfl | ⟨sender, share, hrec⟩ | ⟨c1, c2, hrec⟩
  · exact Or.inl rfl
  · obtain ⟨m, -, hm⟩ := (Trace.received_history _).mp hrec
    exact Or.inr (you_proposals hm)
  · obtain ⟨m, -, hm⟩ := (Trace.received_history _).mp hrec
    match m, hm with
    | 0, hm | 1, hm | 2, hm | 3, hm | 4, hm | 5, hm | 6, hm | _ + 7, hm => cases hm

theorem wtree_resolves (hv : ∀ b, BlockValid b) (hcf : CollisionFree) :
    Resolves cfg wtree (net hv) := by
  have hne : ∀ b b' : Block, b ≠ b' → blockHash b ≠ blockHash b' := fun b b' h he =>
    h (hcf b b' he)
  intro k h n b hb
  have hheld : b = anchorB ∨ b = blk1 ∨ b = blk2 := by
    rcases honest_cases h with rfl | rfl
    · exact me_holds (by have := hb; simp only [net, tr_me] at this; exact this)
    · exact you_holds (by have := hb; simp only [net, tr_you] at this; exact this)
  unfold wtree
  rcases hheld with rfl | rfl | rfl
  · rw [ite_eq_right (hne _ _ (by decide)), ite_eq_right (hne _ _ (by decide)), ite_eq_left rfl]
  · rw [ite_eq_right (hne _ _ (by decide)), ite_eq_left rfl]
  · rw [ite_eq_left rfl]

/-- `me` is honest in epoch one. -/
theorem me_honest1 : com.Honest me := ⟨⟨1⟩, rfl⟩

/-- `you` is honest in epoch two. -/
theorem you_honest2 : com.Honest you := ⟨⟨2⟩, rfl⟩

/--
**Every premise of `NoFork` holds, with blocks of two epochs committed and decided.**

The configuration is coherent, and the tree is coherent and holds what the nodes
hold. The first block, the last of epoch one, has a backed `Cert2`. The second
block opens epoch two, `you` voted1 for it behind that `Cert2`
(`OpensEpochJustified`), and it has a backed `Cert1` and `Cert2` of its own. So
`NoFork` relates two `Cert2`s of different epochs, and `DecideAgreement` and
`DecidesValid` two decided blocks: `me` decided the first, `you` the second.
-/
theorem premises_met (hv : ∀ b, BlockValid b) (hcf : CollisionFree) :
    ConfigCoherent cfg ∧ TreeCoherent wtree ∧ Resolves cfg wtree (net hv)
      ∧ Cert2Backed (net hv).trace cert2₁ ∧ Cert2Backed (net hv).trace cert2₂
      ∧ cert2₁.data.epoch < cert2₂.data.epoch
      ∧ EntersEpoch cfg blk2 ∧ SentBy ((net hv).trace you you_honest2) (.vote1 voteYou)
      ∧ DecidedBlock cfg (net hv) me me_honest1 blk1 ∧ DecidedBlock cfg (net hv) you you_honest2 blk2 :=
  ⟨cfg_coherent, wtree_coherent, wtree_resolves hv hcf, backed2, backed2', by decide,
    ⟨by decide, by decide, by decide⟩,
    by show SentBy (tr you) _; rw [tr_you]; exact ⟨3, by simp [yours]⟩,
    ⟨4, [blk1], cert1₁, cert2₁, by show _ ∈ (tr me 4).output; rw [tr_me]; simp [script], by simp, rfl⟩,
    ⟨6, [blk2], cert1₂, cert2₂, by show _ ∈ (tr you 6).output; rw [tr_you]; simp [yours], by simp, rfl⟩⟩

/--
**And honesty is per epoch.** `me` is honest in epoch one and not in epoch two,
where `you` is. In the second block's view `me` votes1 for two different blocks,
so its trace breaks the signing rules taken over every epoch; it meets them for
the epoch it is honest in, which is all `Network.safe` asks.
-/
theorem per_epoch (hcf : CollisionFree) :
    com.honest ⟨1⟩ me ∧ ¬ com.honest ⟨2⟩ me ∧ com.honest ⟨2⟩ you
      ∧ SentBy script (.vote1 vote1₂) ∧ SentBy script (.vote1 bad)
      ∧ vote1₂.view = bad.view ∧ vote1₂ ≠ bad
      ∧ ¬ SafeHistory cfg me (fun _ => True) (script.history 7) := by
  have hne : vote1₂ ≠ bad := fun h => by
    have hh : blockHash blk2 = blockHash blk2' := congrArg (fun v : Vote1 => v.data.blockHash) h
    exact absurd (congrArg Proposal.identity (hcf _ _ hh)) (by decide)
  refine ⟨rfl, fun h => absurd (me_honest h) (by decide), rfl, ⟨6, by simp [script]⟩,
    ⟨6, by simp [script]⟩, rfl, hne, fun hs => hne ?_⟩
  exact hs.vote1Once vote1₂ bad ((Trace.sent_history _).mpr ⟨6, by omega, by simp [script]⟩)
    ((Trace.sent_history _).mpr ⟨6, by omega, by simp [script]⟩) trivial rfl rfl

end Witness
end NewProtocol
