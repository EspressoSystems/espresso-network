module

public import NewProtocolImpl.Machine
public import NewProtocolImpl.Steps
public import NewProtocolSpec.Timing
public import NewProtocolSpec.Properties

/-!
# The machine meets the specification

`historyOf_protocol`: every history the machine produces obeys `ProtocolHistory`,
given that validity reports are truthful.
`historyOf_settled`: after every step nothing is owed, so the machine is `Prompt`
for every `δ` (`prompt_of_machine`).

The argument has three parts. What each round of `discharge` emits is justified by
the obligation it discharges, which reads only the inputs, so it stays justified
as the step's outputs grow (`StepOK`). Each emission keeps the rules about
everything sent (`Global`), because an obligation is owed only while nothing sent
conflicts with it. And the pass leaves nothing owed: an obligation passed over
was not owed, and more output never makes one owed (`owed_grow`); one taken is
no longer owed; and every obligation is among the candidates.
-/

@[expose] public section

namespace NewProtocolImpl

open NewProtocol NewProtocol.Lists History

variable {cfg : Config} {leader : EpochNumber → ViewNumber → Option PubKey} {node : PubKey}

/-! ## What each part of a step outputs -/

theorem find?_spec {α : Type} {l : List α} {p : α → Bool} {a : α} (h : l.find? p = some a) :
    a ∈ l ∧ p a = true :=
  ⟨List.mem_of_find?_eq_some h, List.find?_some h⟩

/-! ## The decide walk -/

theorem chainFrom_head (h : History) (seen : List ViewNumber) : ∀ (fuel : Nat) (b : Block),
    ∃ rest, chainFrom cfg h seen fuel b = b :: rest
  | 0, b => ⟨[], rfl⟩
  | fuel + 1, b => by
    simp only [chainFrom]
    split
    · exact ⟨_, rfl⟩
    · exact ⟨[], rfl⟩

theorem chainFrom_linked (h : History) (seen : List ViewNumber) : ∀ (fuel : Nat) (b : Block),
    ChainLinked (chainFrom cfg h seen fuel b)
  | 0, b => trivial
  | fuel + 1, b => by
    simp only [chainFrom]
    split
    · rename_i q hq
      obtain ⟨_, hd⟩ := find?_spec hq
      obtain ⟨hv, hh, _⟩ := of_decide_eq_true hd
      obtain ⟨rest, hr⟩ := chainFrom_head (cfg := cfg) h seen fuel q
      have hl := chainFrom_linked h seen fuel q
      rw [hr] at hl ⊢
      exact ⟨hv, hh.symm, hl⟩
    · trivial

/-- Below its newest block, a decide's blocks are held, after genesis, earlier, and not decided or seen. -/
theorem mem_chainFrom (h : History) (seen : List ViewNumber) : ∀ (fuel : Nat) (b x : Block),
    x ∈ chainFrom cfg h seen fuel b →
    x = b ∨ (x ∈ proposalsHeld cfg h ∧ cfg.anchorView < x.viewNumber ∧ x.viewNumber ∉ decidedViews h
      ∧ x.viewNumber ∉ seen ∧ x.viewNumber.toNat < b.viewNumber.toNat)
  | 0, b, x, hx => Or.inl (List.mem_singleton.mp hx)
  | fuel + 1, b, x, hx => by
    simp only [chainFrom] at hx
    split at hx
    · rename_i q hq
      rcases List.mem_cons.mp hx with rfl | hx
      · exact Or.inl rfl
      · obtain ⟨hmem, hd⟩ := find?_spec hq
        obtain ⟨_, _, hg, hnd, hlt, hns⟩ := of_decide_eq_true hd
        have hlt' : q.viewNumber.toNat < b.viewNumber.toNat := hlt
        rcases mem_chainFrom h seen fuel q x hx with rfl | ⟨hx, hg', hnd', hns', hlt''⟩
        · exact Or.inr ⟨hmem, hg, hnd, hns, hlt'⟩
        · exact Or.inr ⟨hx, hg', hnd', hns', by omega⟩
    · exact Or.inl (List.mem_singleton.mp hx)

/-- A decide's blocks are at distinct views. -/
theorem chainFrom_nodup (h : History) (seen : List ViewNumber) : ∀ (fuel : Nat) (b : Block),
    ((chainFrom cfg h seen fuel b).map (·.viewNumber)).Nodup
  | 0, b => by simp [chainFrom]
  | fuel + 1, b => by
    simp only [chainFrom]
    split
    · rename_i q hq
      obtain ⟨_, hd⟩ := find?_spec hq
      obtain ⟨_, _, _, _, hlt, _⟩ := of_decide_eq_true hd
      have hlt' : q.viewNumber.toNat < b.viewNumber.toNat := hlt
      refine List.nodup_cons.mpr ⟨fun hm => ?_, chainFrom_nodup h seen fuel q⟩
      obtain ⟨x, hx, hxv⟩ := List.mem_map.mp hm
      have hxv' : x.viewNumber.toNat = b.viewNumber.toNat := congrArg ViewNumber.toNat hxv
      rcases mem_chainFrom h seen fuel q x hx with rfl | ⟨-, -, -, -, hlt''⟩ <;> omega
    · simp

/-- The views a list of outputs decides. -/
def deliveredViews (out : List Output) : List ViewNumber :=
  out.flatMap fun o => match o with
    | .decided blocks _ _ => blocks.map (·.viewNumber)
    | _ => []

theorem deliveredViews_cons_decided {blocks : List Block} {c1 : Cert1} {c2 : Cert2} {out : List Output} :
    deliveredViews (.decided blocks c1 c2 :: out) = blocks.map (·.viewNumber) ++ deliveredViews out := rfl

theorem deliveredViews_append {out out' : List Output} :
    deliveredViews (out ++ out') = deliveredViews out ++ deliveredViews out' := by
  simp [deliveredViews, List.flatMap_append]

/-- What a history whose last step output `out` decided: what came before, or `out`. -/
theorem decidedView_snoc {pre : History} {i : Input} {out : List Output} {v : ViewNumber} :
    (pre ++ [Step.mk i out]).DecidedView v ↔ v ∈ decidedViews pre ∨ v ∈ deliveredViews out := by
  rw [← mem_decidedViews]
  simp only [decidedViews, List.flatMap_append, List.mem_append, List.flatMap_cons, List.flatMap_nil,
    List.append_nil]
  apply or_congr Iff.rfl
  simp only [deliveredViews, decidesOf, List.mem_flatMap, List.mem_filterMap]
  constructor
  · rintro ⟨d, ⟨o, ho, hod⟩, hv⟩
    refine ⟨o, ho, ?_⟩
    cases o <;> simp at hod
    subst hod; exact hv
  · rintro ⟨o, ho, hv⟩
    cases o with
    | send m => simp at hv
    | decided blocks c1 c2 => exact ⟨(blocks, c1, c2), ⟨_, ho, rfl⟩, hv⟩

theorem mem_decidesFor {h : History} {c : Cert2} {c1 : Block → Cert1} :
    ∀ {seen : List ViewNumber} {bs : List Block} {x : Output}, x ∈ decidesFor cfg h c c1 seen bs →
      ∃ b ∈ bs, ∃ seen', x = .decided (chainFrom cfg h seen' b.viewNumber.toNat b) (c1 b) c
  | _, [], _, hx => by simp [decidesFor] at hx
  | seen, b :: bs, x, hx => by
    simp only [decidesFor] at hx
    split at hx
    · obtain ⟨b', hb', s', he⟩ := mem_decidesFor hx
      exact ⟨b', List.mem_cons_of_mem _ hb', s', he⟩
    · rcases List.mem_cons.mp hx with rfl | hx
      · exact ⟨b, List.mem_cons_self, seen, rfl⟩
      · obtain ⟨b', hb', s', he⟩ := mem_decidesFor hx
        exact ⟨b', List.mem_cons_of_mem _ hb', s', he⟩

/--
The decides a `Cert2` calls for deliver each view once: none at a view decided
already, seen, or delivered by an earlier one of them.
-/
theorem decidesFor_once {h : History} {c : Cert2} {c1 : Block → Cert1} :
    ∀ (seen : List ViewNumber) (bs : List Block) (j : Nat) (blocks : List Block) (c1' : Cert1) (c2 : Cert2),
      (decidesFor cfg h c c1 seen bs)[j]? = some (.decided blocks c1' c2) →
      (blocks.map (·.viewNumber)).Nodup ∧ ∀ b ∈ blocks, b.viewNumber ∉ seen
        ∧ b.viewNumber ∉ decidedViews h
        ∧ b.viewNumber ∉ deliveredViews ((decidesFor cfg h c c1 seen bs).take j)
  | _, [], _, _, _, _, hj => by simp [decidesFor] at hj
  | seen, b :: bs, j, blocks, c1', c2, hj => by
    simp only [decidesFor] at hj ⊢
    split at hj
    · rename_i hskip
      rw [ite_eq_left hskip]
      exact decidesFor_once seen bs j blocks c1' c2 hj
    · rename_i hnew
      rw [ite_eq_right hnew]
      have hnew' : b.viewNumber ∉ seen ∧ b.viewNumber ∉ decidedViews h := not_or.mp hnew
      cases j with
      | zero =>
        simp only [List.getElem?_cons_zero, Option.some.injEq, Output.decided.injEq] at hj
        obtain ⟨rfl, -, -⟩ := hj
        refine ⟨chainFrom_nodup _ _ _ _, fun x hx => ?_⟩
        simp only [List.take_zero, deliveredViews, List.flatMap_nil, List.not_mem_nil, not_false_eq_true,
          and_true]
        rcases mem_chainFrom _ _ _ _ _ hx with rfl | ⟨-, -, hnd, hns, -⟩
        · exact hnew'
        · exact ⟨hns, hnd⟩
      | succ j =>
        simp only [List.getElem?_cons_succ] at hj
        obtain ⟨hnd, hall⟩ := decidesFor_once _ bs j blocks c1' c2 hj
        refine ⟨hnd, fun x hx => ?_⟩
        obtain ⟨hs, hd, hdl⟩ := hall x hx
        simp only [List.mem_append, not_or] at hs
        refine ⟨hs.1, hd, ?_⟩
        rw [List.take_succ_cons, deliveredViews_cons_decided, List.mem_append, not_or]
        exact ⟨hs.2, hdl⟩

/-- Every head the decides went over ends up at a view decided, seen, or delivered by them. -/
theorem decidesFor_covers {h : History} {c : Cert2} {c1 : Block → Cert1} :
    ∀ (seen : List ViewNumber) (bs : List Block), ∀ b ∈ bs,
      b.viewNumber ∈ seen ∨ b.viewNumber ∈ decidedViews h
        ∨ b.viewNumber ∈ deliveredViews (decidesFor cfg h c c1 seen bs)
  | _, [], _, hb => by simp at hb
  | seen, b :: bs, x, hx => by
    simp only [decidesFor]
    split
    · rename_i hskip
      rcases List.mem_cons.mp hx with rfl | hx
      · rcases hskip with h | h
        · exact Or.inl h
        · exact Or.inr (Or.inl h)
      · exact decidesFor_covers seen bs x hx
    · rcases List.mem_cons.mp hx with rfl | hx
      · obtain ⟨rest, hr⟩ := chainFrom_head (cfg := cfg) h seen x.viewNumber.toNat x
        refine Or.inr (Or.inr ?_)
        rw [deliveredViews_cons_decided, hr]
        exact List.mem_append_left _ List.mem_cons_self
      · rcases decidesFor_covers _ bs x hx with h | h | h
        · rcases List.mem_append.mp h with h | h
          · exact Or.inl h
          · exact Or.inr (Or.inr (by rw [deliveredViews_cons_decided]; exact List.mem_append_left _ h))
        · exact Or.inr (Or.inl h)
        · exact Or.inr (Or.inr (by rw [deliveredViews_cons_decided]; exact List.mem_append_right _ h))

/-- A decide obligation sends no message. -/
theorem send_not_mem_act_decide {h : History} {c : Cert2} {m : Message} :
    Output.send m ∉ act cfg leader node h (.decide c) := fun hx => by
  simp only [act] at hx
  obtain ⟨_, _, _, he⟩ := mem_decidesFor hx
  cases he

/-- What a proposal obligation sends: the first candidate proposal of its epoch, or else the first re-vote request. -/
theorem mem_act_propose {h : History} {e : EpochNumber} {w : ViewNumber} {m : Message}
    (hx : Output.send m ∈ act cfg leader node h (.propose e w)) :
    (∃ p, m = .proposal p ∧ (proposalCandidates cfg h w).find?
        (fun p => decide (p.epoch = e ∧ ProposalJustifiedB cfg leader node h p)) = some p)
      ∨ ∃ r, m = .revote r ∧ (proposalCandidates cfg h w).find?
          (fun p => decide (p.epoch = e ∧ ProposalJustifiedB cfg leader node h p)) = none
        ∧ (revoteCandidates cfg h w).find?
          (fun r => decide (r.cert.data.epoch = e ∧ RevoteJustifiedB cfg leader node h r)) = some r := by
  simp only [act] at hx
  split at hx
  · rename_i p hp
    exact Or.inl ⟨p, Output.send.inj (List.mem_singleton.mp hx), hp⟩
  · rename_i hp
    split at hx
    · rename_i r hr
      exact Or.inr ⟨r, Output.send.inj (List.mem_singleton.mp hx), hp, hr⟩
    · cases hx

theorem act_subsingleton {h : History} {o : Obligation} {m m' : Message}
    (hx : Output.send m ∈ act cfg leader node h o) (hy : Output.send m' ∈ act cfg leader node h o) :
    m = m' := by
  cases o with
  | vote1 p => simp only [act, List.mem_singleton, Output.send.injEq] at hx hy; rw [hx, hy]
  | vote1Again r => simp only [act, List.mem_singleton, Output.send.injEq] at hx hy; rw [hx, hy]
  | vote2 c => simp only [act, List.mem_singleton, Output.send.injEq] at hx hy; rw [hx, hy]
  | decide c => exact absurd hx send_not_mem_act_decide
  | propose e w =>
    rcases mem_act_propose hx with ⟨p, rfl, hp⟩ | ⟨r, rfl, hp, hr⟩ <;>
      rcases mem_act_propose hy with ⟨p', rfl, hp'⟩ | ⟨r', rfl, hp', hr'⟩
    · rw [hp] at hp'; cases hp'; rfl
    · rw [hp] at hp'; cases hp'
    · rw [hp] at hp'; cases hp'
    · rw [hr] at hr'; cases hr'; rfl

theorem act_vote1 {h : History} {o : Obligation} {v : Vote1}
    (hm : Output.send (.vote1 v) ∈ act cfg leader node h o) :
    (∃ p, o = .vote1 p ∧ v = ⟨⟨blockHash p, p.epoch, p.blockHeader.blockNumber⟩, p.viewNumber, node⟩)
      ∨ ∃ r, o = .vote1Again r ∧ v = ⟨r.cert.data, r.view, node⟩ := by
  cases o with
  | vote1 p => simp only [act, List.mem_singleton, Output.send.injEq, Message.vote1.injEq] at hm
               exact Or.inl ⟨p, rfl, hm⟩
  | vote1Again r => simp only [act, List.mem_singleton, Output.send.injEq, Message.vote1.injEq] at hm
                    exact Or.inr ⟨r, rfl, hm⟩
  | vote2 c => simp [act] at hm
  | decide c => exact absurd hm send_not_mem_act_decide
  | propose e w =>
    rcases mem_act_propose hm with ⟨_, h, -⟩ | ⟨_, h, -⟩ <;> cases h

theorem act_vote2 {h : History} {o : Obligation} {v : Vote2}
    (hm : Output.send (.vote2 v) ∈ act cfg leader node h o) :
    ∃ c, o = .vote2 c ∧ v = ⟨c.data.toVote2, c.view, node⟩ := by
  cases o with
  | vote1 p => simp [act] at hm
  | vote1Again r => simp [act] at hm
  | vote2 c => simp only [act, List.mem_singleton, Output.send.injEq, Message.vote2.injEq] at hm
               exact ⟨c, rfl, hm⟩
  | decide c => exact absurd hm send_not_mem_act_decide
  | propose e w =>
    rcases mem_act_propose hm with ⟨_, h, -⟩ | ⟨_, h, -⟩ <;> cases h

theorem act_no_timeoutVote {h : History} {o : Obligation} {v : TimeoutVote} :
    Output.send (.timeoutVote v) ∉ act cfg leader node h o := by
  intro hm
  cases o with
  | vote1 p => simp [act] at hm
  | vote1Again r => simp [act] at hm
  | vote2 c => simp [act] at hm
  | decide c => exact absurd hm send_not_mem_act_decide
  | propose e w =>
    rcases mem_act_propose hm with ⟨_, h, -⟩ | ⟨_, h, -⟩ <;> cases h

/-- A proposal the machine sends is a candidate it may make. -/
theorem act_proposal {h : History} {o : Obligation} {p : Proposal}
    (hm : Output.send (.proposal p) ∈ act cfg leader node h o) :
    ∃ e v, o = .propose e v ∧ p ∈ proposalCandidates cfg h v ∧ p.epoch = e
      ∧ ProposalJustifiedB cfg leader node h p := by
  cases o with
  | vote1 q => simp [act] at hm
  | vote1Again r => simp [act] at hm
  | vote2 c => simp [act] at hm
  | decide c => exact absurd hm send_not_mem_act_decide
  | propose e w =>
    rcases mem_act_propose hm with ⟨q, hq, hf⟩ | ⟨r, hr, -⟩
    · cases hq
      obtain ⟨hmem, hj⟩ := find?_spec hf
      obtain ⟨he, hj⟩ := of_decide_eq_true hj
      exact ⟨e, w, rfl, hmem, he, hj⟩
    · cases hr

/-- A re-vote request the machine sends is a candidate it may make. -/
theorem act_revote {h : History} {o : Obligation} {r : RevoteRequest}
    (hm : Output.send (.revote r) ∈ act cfg leader node h o) :
    ∃ e v, o = .propose e v ∧ r ∈ revoteCandidates cfg h v ∧ r.cert.data.epoch = e
      ∧ RevoteJustifiedB cfg leader node h r := by
  cases o with
  | vote1 q => simp [act] at hm
  | vote1Again r => simp [act] at hm
  | vote2 c => simp [act] at hm
  | decide c => exact absurd hm send_not_mem_act_decide
  | propose e w =>
    rcases mem_act_propose hm with ⟨p, hp, -⟩ | ⟨q, hq, -, hf⟩
    · cases hp
    · cases hq
      obtain ⟨hmem, hj⟩ := find?_spec hf
      obtain ⟨he, hj⟩ := of_decide_eq_true hj
      exact ⟨e, w, rfl, hmem, he, hj⟩

/-- A vote1 an obligation calls for is at an epoch and view the node has not voted1 in. -/
theorem act_vote1_fresh {h : History} {o : Obligation} {v : Vote1} (ho : Owed cfg leader node h o)
    (hm : Output.send (.vote1 v) ∈ act cfg leader node h o) :
    ∀ v', h.Sent (.vote1 v') → v'.data.epoch = v.data.epoch → v'.view ≠ v.view := by
  rcases act_vote1 hm with ⟨p, rfl, rfl⟩ | ⟨r, rfl, rfl⟩
  · exact ho.notVoted
  · exact ho.notVoted

theorem mem_timeoutAnswer {h : History} {i : Input} {x : Output}
    (hx : x ∈ timeoutAnswer cfg node h i) :
    ∃ v, x = .send (.timeoutVote ⟨⟨epochOfHistory cfg h, lockOf cfg h⟩, v, node⟩)
      ∧ ((i = .timeout v ∧ viewOf cfg h = v) ∨ (i = .timeoutOneHonest v ∧ viewOf cfg h ≤ v)) := by
  cases i <;> simp only [timeoutAnswer, List.not_mem_nil] at hx
  all_goals (split at hx <;> simp only [List.mem_singleton, List.not_mem_nil] at hx)
  · rename_i v hv; exact ⟨v, hx, Or.inl ⟨rfl, hv⟩⟩
  · rename_i v hv; exact ⟨v, hx, Or.inr ⟨rfl, hv⟩⟩

theorem act_decided {h : History} {o : Obligation} {blocks : List Block} {c1 : Cert1} {c2 : Cert2}
    (hm : Output.decided blocks c1 c2 ∈ act cfg leader node h o) :
    o = .decide c2 ∧ ∃ b ∈ proposalsHeld cfg h, Commits c2 b ∧ cfg.anchorView < b.viewNumber
      ∧ h.HasCert1 cfg c1 ∧ Certifies c1 b ∧ ∃ seen, blocks = chainFrom cfg h seen b.viewNumber.toNat b := by
  cases o with
  | vote1 q => simp [act] at hm
  | vote1Again r => simp [act] at hm
  | vote2 c => simp [act] at hm
  | decide c =>
    simp only [act] at hm
    obtain ⟨b, hb, seen, he⟩ := mem_decidesFor hm
    simp only [Output.decided.injEq] at he
    obtain ⟨rfl, rfl, rfl⟩ := he
    obtain ⟨hb, hd⟩ := List.mem_filter.mp hb
    obtain ⟨hd, hsome⟩ := Bool.and_eq_true_iff.mp hd
    obtain ⟨hcm, hg⟩ := of_decide_eq_true hd
    obtain ⟨c1, hc1⟩ := Option.isSome_iff_exists.mp hsome
    obtain ⟨hmem, hcert⟩ := find?_spec hc1
    rw [hc1]
    exact ⟨rfl, b, hb, hcm, hg, mem_cert1sHeld.mp hmem, of_decide_eq_true hcert, seen, rfl⟩
  | propose e w =>
    simp only [act] at hm; split at hm
    · simp at hm
    · split at hm <;> simp at hm

/-! ## A pass of `discharge` -/

theorem fresh_of_owed {h : History} {o : Obligation} (ho : Owed cfg leader node h o) :
    fresh cfg h o = true := by
  cases o with
  | vote1 p =>
    obtain ⟨-, -, -, -, -, -, -, ht, honce, hin⟩ := ho
    simp only [fresh, List.all_eq_true, bne_iff_ne, ne_eq, Bool.and_eq_true, Bool.or_eq_true, decide_eq_true_eq,
      Bool.not_eq_true', decide_eq_false_iff_not, timedOutB_iff]
    exact ⟨⟨inView_iff_viewOf.mp hin, ht⟩, fun v hv =>
      (Classical.em _).elim (fun he => Or.inr (honce v (mem_vote1sSent.mp hv) he)) Or.inl⟩
  | vote1Again r =>
    obtain ⟨-, -, -, -, -, ht, honce, hin⟩ := ho
    simp only [fresh, List.all_eq_true, bne_iff_ne, ne_eq, Bool.and_eq_true, Bool.or_eq_true, decide_eq_true_eq,
      Bool.not_eq_true', decide_eq_false_iff_not, timedOutB_iff]
    exact ⟨⟨inView_iff_viewOf.mp hin, ht⟩, fun v hv =>
      (Classical.em _).elim (fun he => Or.inr (honce v (mem_vote1sSent.mp hv) he)) Or.inl⟩
  | vote2 c =>
    simp only [fresh, List.all_eq_true, bne_iff_ne, ne_eq, Bool.and_eq_true, Bool.or_eq_true,
      Bool.not_eq_true', decide_eq_false_iff_not, timedOutB_iff]
    exact ⟨fun ht => ho.notPast (Or.inl ht), fun v hv =>
      (Classical.em _).elim (fun he => Or.inr (ho.notVoted v (mem_vote2sSent.mp hv) he)) Or.inl⟩
  | decide c =>
    obtain ⟨b, -, -, -, hcm, -, -, -, hg, hw⟩ := ho
    simp only [fresh, decide_eq_true_eq, aboveFloorB_iff]
    exact ⟨Nat.lt_of_lt_of_le hg hcm.1, fun w hd => Nat.lt_of_lt_of_le (hw w hd) hcm.1⟩
  | propose e v =>
    obtain ⟨-, hnp, ht, hin⟩ := ho
    simp only [fresh, List.all_eq_true, bne_iff_ne, ne_eq, Bool.and_eq_true, Bool.or_eq_true, decide_eq_true_eq,
      Bool.not_eq_true', decide_eq_false_iff_not, timedOutB_iff]
    exact ⟨⟨⟨inView_iff_viewOf.mp hin, ht⟩, fun p hp => (Classical.em _).elim
        (fun hv => Or.inr fun he => hnp (Or.inl ⟨p, mem_proposalsSent.mp hp, hv, he⟩)) Or.inl⟩,
      fun r hr => (Classical.em _).elim
        (fun hv => Or.inr fun he => hnp (Or.inr ⟨r, mem_revotesSent.mp hr, hv, he⟩)) Or.inl⟩

section Discharge

variable {pre : History} {i : Input}

theorem discharge_sup : ∀ (obs : List Obligation) (out : List Output),
    ∀ x ∈ out, x ∈ discharge cfg leader node pre i obs out
  | [], _, _, hx => hx
  | o :: os, out, x, hx => by
    simp only [discharge]
    split
    · exact discharge_sup os _ x (List.mem_append_left _ hx)
    · exact discharge_sup os _ x hx

theorem mem_discharge : ∀ (obs : List Obligation) (out : List Output) (x : Output),
    x ∈ discharge cfg leader node pre i obs out →
    x ∈ out ∨ ∃ o ∈ obs, ∃ out', (∀ y ∈ out, y ∈ out') ∧ Owed cfg leader node (pre ++ [Step.mk i out']) o
      ∧ x ∈ act cfg leader node (pre ++ [Step.mk i out']) o
  | [], _, _, hx => Or.inl hx
  | o :: os, out, x, hx => by
    simp only [discharge] at hx
    split at hx
    · rename_i howed
      rcases mem_discharge os _ x hx with hx | ⟨o', ho', out', hsub, h1, h2⟩
      · rcases List.mem_append.mp hx with hx | hx
        · exact Or.inl hx
        · exact Or.inr ⟨o, List.mem_cons_self, out, fun _ h => h, howed.2, hx⟩
      · exact Or.inr ⟨o', List.mem_cons_of_mem _ ho', out',
          fun y hy => hsub y (List.mem_append_left _ hy), h1, h2⟩
    · rcases mem_discharge os _ x hx with hx | ⟨o', ho', out', hsub, h1, h2⟩
      · exact Or.inl hx
      · exact Or.inr ⟨o', List.mem_cons_of_mem _ ho', out', hsub, h1, h2⟩

/-- What the last step of `pre ++ [Step.mk i (out ++ more)]` sent: what it sent before, or `more`. -/
theorem sent_more {out more : List Output} {m : Message} :
    (pre ++ [Step.mk i (out ++ more)]).Sent m
      ↔ (pre ++ [Step.mk i out]).Sent m ∨ Output.send m ∈ more := by
  simp only [sent_snoc, List.mem_append]
  exact or_assoc.symm

/-- An obligation taken is no longer owed. -/
theorem not_owed_after_act {out : List Output} {o : Obligation}
    (ho : Owed cfg leader node (pre ++ [Step.mk i out]) o) :
    ¬ Owed cfg leader node
      (pre ++ [Step.mk i (out ++ act cfg leader node (pre ++ [Step.mk i out]) o)]) o := by
  intro hn
  cases o with
  | vote1 p =>
    obtain ⟨-, -, -, -, -, -, -, -, honce, -⟩ := hn
    exact honce ⟨⟨blockHash p, p.epoch, p.blockHeader.blockNumber⟩, p.viewNumber, node⟩
      (sent_more.mpr (Or.inr (by simp [act]))) rfl rfl
  | vote1Again r =>
    obtain ⟨-, -, -, -, -, -, honce, -⟩ := hn
    exact honce ⟨r.cert.data, r.view, node⟩ (sent_more.mpr (Or.inr (by simp [act]))) rfl rfl
  | vote2 c =>
    exact hn.notVoted ⟨c.data.toVote2, c.view, node⟩ (sent_more.mpr (Or.inr (by simp [act]))) rfl rfl
  | decide c =>
    obtain ⟨b, c1, -, hb, hcm, hc1, hcert, hnd, hfl⟩ := hn
    have hb' : b ∈ proposalsHeld cfg (pre ++ [Step.mk i out]) :=
      mem_proposalsHeld.mpr ((sameInputs_grow.hasProposal b).mpr hb)
    have hsome : (cert1For cfg (pre ++ [Step.mk i out]) b).isSome := List.find?_isSome.mpr
      ⟨c1, mem_cert1sHeld.mpr ((sameInputs_grow.hasCert1 c1).mpr hc1), decide_eq_true hcert⟩
    apply hnd
    rw [decidedView_snoc, deliveredViews_append]
    rcases decidesFor_covers (cfg := cfg) (h := pre ++ [Step.mk i out]) (c := c)
        (c1 := fun b => (cert1For cfg (pre ++ [Step.mk i out]) b).getD cfg.anchorCert) []
        ((proposalsHeld cfg (pre ++ [Step.mk i out])).filter fun b =>
          decide (Commits c b ∧ cfg.anchorView < b.viewNumber) && (cert1For cfg (pre ++ [Step.mk i out]) b).isSome) b
        (List.mem_filter.mpr ⟨hb', Bool.and_eq_true_iff.mpr ⟨decide_eq_true ⟨hcm, hfl.1⟩, hsome⟩⟩)
      with h0 | hd | hd
    · simp at h0
    · rcases decidedView_snoc.mp (mem_decidedViews.mp hd) with h | h
      · exact Or.inl h
      · exact Or.inr (List.mem_append_left _ h)
    · exact Or.inr (List.mem_append_right _ hd)
  | propose e v =>
    have hnp := hn.2.1
    cases hf : (proposalCandidates cfg (pre ++ [Step.mk i out]) v).find?
        (fun p => decide (p.epoch = e ∧ ProposalJustifiedB cfg leader node (pre ++ [Step.mk i out]) p)) with
    | some q' =>
      obtain ⟨hmem, hd⟩ := find?_spec hf
      exact hnp (Or.inl ⟨q', sent_more.mpr (Or.inr (by simp only [act]; rw [hf]; exact List.mem_singleton_self _)),
        viewNumber_of_mem_candidates hmem, (of_decide_eq_true hd).1⟩)
    | none =>
      rw [List.find?_eq_none] at hf
      rcases (owedB_iff.mpr ho).1 with ⟨q, hq, he, hj⟩ | ⟨r, hr, he, hj⟩
      · exact hf q hq (decide_eq_true ⟨he, hj⟩)
      cases hf2 : (revoteCandidates cfg (pre ++ [Step.mk i out]) v).find?
          (fun r => decide (r.cert.data.epoch = e ∧ RevoteJustifiedB cfg leader node (pre ++ [Step.mk i out]) r)) with
      | none =>
        rw [List.find?_eq_none] at hf2
        exact hf2 r hr (decide_eq_true ⟨he, hj⟩)
      | some r' =>
        obtain ⟨hmem, hd⟩ := find?_spec hf2
        have hf0 : (proposalCandidates cfg (pre ++ [Step.mk i out]) v).find?
            (fun p => decide (p.epoch = e ∧ ProposalJustifiedB cfg leader node (pre ++ [Step.mk i out]) p)) = none :=
          List.find?_eq_none.mpr hf
        exact hnp (Or.inr ⟨r', sent_more.mpr (Or.inr (by simp only [act]; rw [hf0, hf2]; exact List.mem_singleton_self _)),
          view_of_mem_revoteCandidates hmem, (of_decide_eq_true hd).1⟩)

/-- No decide among the outputs `out` of a step delivers a view twice, or one decided before it. -/
def DecidesOnce (pre : History) (i : Input) (out : List Output) : Prop :=
  ∀ j blocks c1 c2, out[j]? = some (.decided blocks c1 c2) →
    (blocks.map (·.viewNumber)).Nodup ∧ ∀ b ∈ blocks, ¬ (pre ++ [Step.mk i (out.take j)]).DecidedView b.viewNumber

/-- Taking an obligation keeps every view decided once. -/
theorem decidesOnce_act {out : List Output} {o : Obligation} (hg : DecidesOnce pre i out) :
    DecidesOnce pre i (out ++ act cfg leader node (pre ++ [Step.mk i out]) o) := by
  intro j blocks c1 c2 hj
  by_cases hjl : j < out.length
  · rw [List.getElem?_append_left hjl] at hj
    rw [List.take_append_of_le_length (by omega)]
    exact hg j blocks c1 c2 hj
  · rw [List.getElem?_append_right (by omega)] at hj
    obtain ⟨rfl, -⟩ := act_decided (List.mem_of_getElem? hj)
    obtain ⟨hnd, hall⟩ := decidesFor_once (cfg := cfg) [] _ (j - out.length) blocks c1 c2 hj
    refine ⟨hnd, fun b hb hd => ?_⟩
    obtain ⟨-, hnd', hnl⟩ := hall b hb
    rw [List.take_append, List.take_of_length_le (by omega), decidedView_snoc,
      deliveredViews_append, List.mem_append] at hd
    rcases hd with hd | hd | hd
    · exact hnd' (mem_decidedViews.mpr (decidedView_snoc.mpr (Or.inl hd)))
    · exact hnd' (mem_decidedViews.mpr (decidedView_snoc.mpr (Or.inr hd)))
    · exact hnl hd

/-- A pass keeps every view decided once. -/
theorem discharge_once : ∀ (obs : List Obligation) (out : List Output), DecidesOnce pre i out →
    DecidesOnce pre i (discharge cfg leader node pre i obs out)
  | [], _, h => h
  | o :: os, out, h => by
    simp only [discharge]
    split
    · exact discharge_once os _ (decidesOnce_act h)
    · exact discharge_once os _ h

/-- A pass leaves owed none of the obligations it went over, nor any it had already cleared. -/
theorem discharge_settles : ∀ (obs : List Obligation) (out : List Output) (done : List Obligation),
    (∀ o ∈ done, ¬ Owed cfg leader node (pre ++ [Step.mk i out]) o) →
    ∀ o, o ∈ done ∨ o ∈ obs →
      ¬ Owed cfg leader node (pre ++ [Step.mk i (discharge cfg leader node pre i obs out)]) o
  | [], out, done, hd, o, ho => by
    rcases ho with ho | ho
    · exact hd o ho
    · simp at ho
  | o' :: os, out, done, hd, o, ho => by
    simp only [discharge]
    split
    · rename_i howed
      apply discharge_settles os _ (o' :: done) _ o
      · rcases ho with ho | ho
        · exact Or.inl (List.mem_cons_of_mem _ ho)
        · rcases List.mem_cons.mp ho with rfl | ho
          · exact Or.inl List.mem_cons_self
          · exact Or.inr ho
      · intro o'' ho''
        rcases List.mem_cons.mp ho'' with rfl | ho''
        · exact not_owed_after_act howed.2
        · exact fun hn => hd o'' ho'' (owed_grow (fun x hx => List.mem_append_left _ hx) hn)
    · rename_i hnot
      apply discharge_settles os out (o' :: done) _ o
      · rcases ho with ho | ho
        · exact Or.inl (List.mem_cons_of_mem _ ho)
        · rcases List.mem_cons.mp ho with rfl | ho
          · exact Or.inl List.mem_cons_self
          · exact Or.inr ho
      · intro o'' ho''
        rcases List.mem_cons.mp ho'' with rfl | ho''
        · exact fun ho => hnot ⟨fresh_of_owed ho, ho⟩
        · exact hd o'' ho''

end Discharge

/-! ## The rules about everything sent -/

section Global

variable {pre : History} {i : Input}

/-- The step's input arriving, with only timeout votes sent yet, keeps `Global`. -/
theorem global_arrive {out : List Output} (hg : Global pre)
    (hout : ∀ x ∈ out, ∃ v, x = .send (.timeoutVote v)) : Global (pre ++ [Step.mk i out]) := by
  have hsent : ∀ m, (∀ v, m ≠ .timeoutVote v) → ((pre ++ [Step.mk i out]).Sent m ↔ pre.Sent m) := by
    intro m hm
    rw [sent_snoc]
    constructor
    · rintro (hs | hs)
      · exact hs
      · obtain ⟨v, hv⟩ := hout _ hs
        exact absurd (Output.send.inj hv) (hm v)
    · exact Or.inl
  have h1 : ∀ v, (pre ++ [Step.mk i out]).Sent (.vote1 v) ↔ pre.Sent (.vote1 v) :=
    fun v => hsent _ (fun _ h => by cases h)
  have h2 : ∀ v, (pre ++ [Step.mk i out]).Sent (.vote2 v) ↔ pre.Sent (.vote2 v) :=
    fun v => hsent _ (fun _ h => by cases h)
  have hp : ∀ p, (pre ++ [Step.mk i out]).Sent (.proposal p) ↔ pre.Sent (.proposal p) :=
    fun p => hsent _ (fun _ h => by cases h)
  have hr : ∀ r, (pre ++ [Step.mk i out]).Sent (.revote r) ↔ pre.Sent (.revote r) :=
    fun r => hsent _ (fun _ h => by cases h)
  refine ⟨?_, ?_, ?_, ?_⟩
  · intro v v' hs hs' he hv; exact hg.vote1Once v v' ((h1 v).mp hs) ((h1 v').mp hs') he hv
  · intro v v' hs hs' he hv; exact hg.vote2Once v v' ((h2 v).mp hs) ((h2 v').mp hs') he hv
  · intro q q' hs hs' he hv; exact hg.proposeOnce q q' ((hp q).mp hs) ((hp q').mp hs') he hv
  · intro r hs
    obtain ⟨h1, h2⟩ := hg.revoteOnce r ((hr r).mp hs)
    exact ⟨fun r' hs' he hv => h1 r' ((hr r').mp hs') he hv, fun p hs' => h2 p ((hp p).mp hs')⟩

/-- Taking an owed obligation keeps `Global`. -/
theorem global_act {out : List Output} {o : Obligation}
    (hg : Global (pre ++ [Step.mk i out])) (ho : Owed cfg leader node (pre ++ [Step.mk i out]) o) :
    Global (pre ++ [Step.mk i (out ++ act cfg leader node (pre ++ [Step.mk i out]) o)]) := by
  refine ⟨?_, ?_, ?_, ?_⟩
  · intro v v' hs hs' he hv
    rcases sent_more.mp hs with a | a <;> rcases sent_more.mp hs' with b | b
    · exact hg.vote1Once v v' a b he hv
    · exact absurd hv (act_vote1_fresh ho b v a he)
    · exact absurd hv.symm (act_vote1_fresh ho a v' b he.symm)
    · have := act_subsingleton a b
      simp only [Message.vote1.injEq] at this
      exact this
  · intro v v' hs hs' he hv
    rcases sent_more.mp hs with a | a <;> rcases sent_more.mp hs' with b | b
    · exact hg.vote2Once v v' a b he hv
    · obtain ⟨c, rfl, rfl⟩ := act_vote2 b
      exact absurd hv (ho.notVoted v a he)
    · obtain ⟨c, rfl, rfl⟩ := act_vote2 a
      exact absurd hv.symm (ho.notVoted v' b he.symm)
    · have := act_subsingleton a b
      simp only [Message.vote2.injEq] at this
      exact this
  · intro q q' hs hs' he hv
    rcases sent_more.mp hs with a | a <;> rcases sent_more.mp hs' with b | b
    · exact hg.proposeOnce q q' a b he hv
    · obtain ⟨e, w, rfl, hmem, hqe, -⟩ := act_proposal b
      exact absurd (Or.inl ⟨q, a, hv.trans (viewNumber_of_mem_candidates hmem), he.trans hqe⟩) ho.2.1
    · obtain ⟨e, w, rfl, hmem, hqe, -⟩ := act_proposal a
      exact absurd (Or.inl ⟨q', b, hv.symm.trans (viewNumber_of_mem_candidates hmem), he.symm.trans hqe⟩) ho.2.1
    · have := act_subsingleton a b
      simp only [Message.proposal.injEq] at this
      exact this
  · intro r hs
    rcases sent_more.mp hs with a | a
    · obtain ⟨h1, h2⟩ := hg.revoteOnce r a
      refine ⟨fun r' hs' he hv => ?_, fun p hs' he => ?_⟩
      · rcases sent_more.mp hs' with b | b
        · exact h1 r' b he hv
        · obtain ⟨e, w, rfl, hmem, hre, _⟩ := act_revote b
          exact absurd (Or.inr ⟨r, a, hv.symm.trans (view_of_mem_revoteCandidates hmem), he.symm.trans hre⟩) ho.2.1
      · rcases sent_more.mp hs' with b | b
        · exact h2 p b he
        · obtain ⟨e, w, rfl, hmem, hpe, -⟩ := act_proposal b
          exact fun hv => ho.2.1 (Or.inr ⟨r, a, hv.symm.trans (viewNumber_of_mem_candidates hmem), he.symm.trans hpe⟩)
    · obtain ⟨e, w, rfl, hmem, hre, _⟩ := act_revote a
      have hw := view_of_mem_revoteCandidates hmem
      refine ⟨fun r' hs' he hv => ?_, fun p hs' he hv => ?_⟩
      · rcases sent_more.mp hs' with b | b
        · exact absurd (Or.inr ⟨r', b, hv.trans hw, he.trans hre⟩) ho.2.1
        · have := act_subsingleton b a
          simp only [Message.revote.injEq] at this
          exact this
      · rcases sent_more.mp hs' with b | b
        · exact ho.2.1 (Or.inl ⟨p, b, hv.trans hw, he.trans hre⟩)
        · exact absurd (act_subsingleton a b) (by simp)

theorem discharge_global : ∀ (obs : List Obligation) (out : List Output),
    Global (pre ++ [Step.mk i out]) →
    Global (pre ++ [Step.mk i (discharge cfg leader node pre i obs out)])
  | [], _, hg => hg
  | o :: os, out, hg => by
    simp only [discharge]
    split
    · rename_i ho
      exact discharge_global os _ (global_act hg ho.2)
    · exact discharge_global os out hg

end Global

/-! ## Every obligation is a candidate -/

theorem mem_candidates {h h0 : History} (he : SameInputs h h0) {o : Obligation}
    (ho : Owed cfg leader node h o) : o ∈ candidates cfg h0 := by
  simp only [candidates, List.mem_append, List.mem_map, or_assoc]
  cases o with
  | decide c =>
    obtain ⟨_, _, hc, _⟩ := ho
    exact Or.inl ⟨c, mem_cert2sHeld.mpr ((he.hasCert2 c).mp hc), rfl⟩
  | vote2 c =>
    obtain ⟨⟨_, hc, _⟩, _⟩ := ho
    exact Or.inr (Or.inl ⟨c, mem_cert1sHeld.mpr ((he.hasCert1 c).mp hc), rfl⟩)
  | vote1 p =>
    obtain ⟨⟨s, vid, hr, _⟩, _⟩ := ho
    exact Or.inr (Or.inr (Or.inl ⟨(s, p, vid), mem_receivedProposals.mpr ((he.received _).mp hr), rfl⟩))
  | vote1Again r =>
    obtain ⟨⟨s, hr, _⟩, _⟩ := ho
    exact Or.inr (Or.inr (Or.inr (Or.inl ⟨(s, r), mem_receivedRevotes.mpr ((he.received _).mp hr), rfl⟩)))
  | propose e v =>
    obtain ⟨hj, -, -, hin⟩ := ho
    have hv0 : viewOf cfg h0 = v := viewOf_of_inView ((he.inView _).mp hin)
    refine Or.inr (Or.inr (Or.inr (Or.inr ⟨e, ?_, by rw [hv0]⟩)))
    simp only [proposeEpochs, List.mem_eraseDups, List.mem_append, List.mem_map, hv0]
    rcases hj with ⟨p, hp, hpv, hpe⟩ | ⟨r, hr, hrv, hre⟩
    · obtain ⟨q, hq, hqe, -⟩ := proposalCandidates_complete ((he.proposalJustified p).mp hp)
      rw [hpv] at hq
      exact Or.inl ⟨q, hq, hqe.trans hpe⟩
    · have hc := revoteCandidates_complete ((he.revoteJustified r).mp hr)
      rw [hrv] at hc
      exact Or.inr ⟨r, hc, hre⟩

/-! ## One step -/

section Step

variable {pre : History} {i : Input}

theorem step_output :
    (step cfg leader node pre i).output = discharge cfg leader node pre i
      (candidates cfg (pre ++ [Step.mk i []])) (timeoutAnswer cfg node pre i) := rfl

theorem step_input : (step cfg leader node pre i).input = i := rfl

theorem sameInputs_step {out : List Output} :
    SameInputs (pre ++ [Step.mk i out]) (pre ++ [step cfg leader node pre i]) := sameInputs_grow

/-- Whatever a step outputs, it outputs because it was owed, or to answer a timer. -/
theorem step_mem {x : Output} (hx : x ∈ (step cfg leader node pre i).output) :
    x ∈ timeoutAnswer cfg node pre i ∨ ∃ o out', (∀ y ∈ timeoutAnswer cfg node pre i, y ∈ out')
      ∧ Owed cfg leader node (pre ++ [Step.mk i out']) o
      ∧ x ∈ act cfg leader node (pre ++ [Step.mk i out']) o := by
  rw [step_output] at hx
  rcases mem_discharge _ _ _ hx with hx | ⟨o, _, out', hsub, ho, ha⟩
  · exact Or.inl hx
  · exact Or.inr ⟨o, out', hsub, ho, ha⟩

/--
A step's timeout votes are its timer answers, which every history the step's
obligations were judged on already carries: so the step has timed out on a view
only if that history had.
-/
theorem timedOut_step {out' : List Output} (hsub : ∀ y ∈ timeoutAnswer cfg node pre i, y ∈ out')
    {v : ViewNumber} (ht : (pre ++ [step cfg leader node pre i]).TimedOut v) :
    (pre ++ [Step.mk i out']).TimedOut v := by
  obtain ⟨vote, hs, hle⟩ := ht
  refine ⟨vote, ?_, hle⟩
  rcases sent_snoc.mp hs with hs | hs
  · exact sent_snoc.mpr (Or.inl hs)
  · rcases step_mem hs with h0 | ⟨o, out'', _, _, ha⟩
    · exact sent_snoc.mpr (Or.inr (hsub _ h0))
    · exact absurd ha act_no_timeoutVote

theorem step_ok (hvt : ValidityTruthful (pre ++ [Step.mk i []])) :
    StepOK cfg leader node pre (step cfg leader node pre i) := by
  refine ⟨?_, ?_, ?_, discharge_once _ _ fun j blocks c1 c2 hj => ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩
  · intro vote hm
    rcases step_mem hm with h0 | ⟨o, out', hsub, ho, ha⟩
    · obtain ⟨v, hx, _⟩ := mem_timeoutAnswer h0; cases hx
    · rcases act_vote1 ha with ⟨p, rfl, rfl⟩ | ⟨r, rfl, rfl⟩
      · obtain ⟨⟨s, vid, hr, _⟩, hw, hval, _, hsafe, hopen, _⟩ := ho
        refine ⟨rfl, Or.inl ⟨s, p, vid, (sameInputs_step.received _).mp hr, hw, ?_, hsafe,
          (sameInputs_step.opens p).mp hopen, rfl, rfl⟩⟩
        exact hvt _ _ ((sameInputs_grow.received _).mp hval) p rfl
      · obtain ⟨⟨s, hr, _⟩, hw, hsafe, _⟩ := ho
        exact ⟨rfl, Or.inr ⟨s, r, (sameInputs_step.received _).mp hr, hw, hsafe, rfl, rfl⟩⟩
  · intro vote hm
    rcases step_mem hm with h0 | ⟨o, out', hsub, ho, ha⟩
    · obtain ⟨v, hx, _⟩ := mem_timeoutAnswer h0; cases hx
    · obtain ⟨c, rfl, rfl⟩ := act_vote2 ha
      obtain ⟨⟨b, hc, hb, hcert, hpay⟩, _, _, _, hgen⟩ := ho
      exact ⟨rfl, hgen.1, c, b, (sameInputs_step.hasCert1 c).mp hc, (sameInputs_step.hasProposal b).mp hb,
        hcert, (sameInputs_step.hasPayload _ _).mp hpay, rfl, rfl⟩
  · intro blocks c1 c2 hm
    rcases step_mem hm with h0 | ⟨o, out', hsub, ho, ha⟩
    · obtain ⟨v, hx, _⟩ := mem_timeoutAnswer h0; cases hx
    · obtain ⟨rfl, b, hb, hcm, hg, hc1, hcert, seen, rfl⟩ := act_decided ha
      obtain ⟨_, _, hc2, _⟩ := ho
      obtain ⟨rest, hr⟩ := chainFrom_head (cfg := cfg) (pre ++ [Step.mk i out']) seen b.viewNumber.toNat b
      refine ⟨b, rest, hr, (sameInputs_step.hasCert2 c2).mp hc2, hcm, (sameInputs_step.hasCert1 c1).mp hc1, hcert,
        chainFrom_linked _ _ _ _, ?_⟩
      intro x hx
      rcases mem_chainFrom _ _ _ _ _ hx with rfl | ⟨hx, hg', -⟩
      · exact ⟨(sameInputs_step.hasProposal x).mp (mem_proposalsHeld.mp hb), hg⟩
      · exact ⟨(sameInputs_step.hasProposal x).mp (mem_proposalsHeld.mp hx), hg'⟩
  · obtain ⟨v, hx, -⟩ := mem_timeoutAnswer (List.mem_of_getElem? hj); cases hx
  · intro vote hm
    rcases step_mem hm with h0 | ⟨o, out', _, ho, ha⟩
    · obtain ⟨v, hx, _⟩ := mem_timeoutAnswer h0; cases hx
    · rcases act_vote1 ha with ⟨p, rfl, rfl⟩ | ⟨r, rfl, rfl⟩
      · obtain ⟨⟨s, vid, hr, hl, _⟩, -, -, -, -, -, hnb, -, -, hin⟩ := ho
        exact ⟨⟨p.viewNumber, (sameInputs_step.viewGround _).mp hin.1, Nat.le_refl _⟩,
          Or.inl ⟨s, p, vid, (sameInputs_step.received _).mp hr, hl, ⟨rfl, rfl⟩,
            (sameInputs_step.notBehind _).mp hnb⟩⟩
      · obtain ⟨⟨s, hr, hl⟩, -, -, -, hnb, -, -, hin⟩ := ho
        exact ⟨⟨r.view, (sameInputs_step.viewGround _).mp hin.1, Nat.le_refl _⟩,
          Or.inr ⟨s, r, (sameInputs_step.received _).mp hr, hl, ⟨rfl, rfl⟩,
            (sameInputs_step.notBehind _).mp hnb⟩⟩
  · intro vote hm ht
    rcases step_mem hm with h0 | ⟨o, out', hsub, ho, ha⟩
    · obtain ⟨v, hx, _⟩ := mem_timeoutAnswer h0; cases hx
    · obtain ⟨c, rfl, rfl⟩ := act_vote2 ha
      exact ho.notPast (Or.inl (timedOut_step hsub ht))
  · intro vote hm
    rcases step_mem hm with h0 | ⟨o, out', _, _, ha⟩
    · obtain ⟨v, hx, -⟩ := mem_timeoutAnswer h0
      simp only [Output.send.injEq, Message.timeoutVote.injEq] at hx
      subst hx
      exact lockOf_lockedOn (cfg := cfg) pre
    · exact absurd ha act_no_timeoutVote
  · intro vote hm
    rcases step_mem hm with h0 | ⟨o, out', _, _, ha⟩
    · obtain ⟨v, hx, hin⟩ := mem_timeoutAnswer h0
      simp only [Output.send.injEq, Message.timeoutVote.injEq] at hx
      subst hx
      refine ⟨rfl, inEpoch_epochOfHistory cfg pre, ?_, ?_⟩
      · exact hasCert1_of_lockable_upTo (m := pre.length)
          (by rw [History.upTo, List.take_length]; exact (lockOf_lockedOn (cfg := cfg) pre).1)
      · rcases hin with ⟨hi, hv⟩ | ⟨hi, hv⟩
        · exact Or.inl ⟨hi, hv ▸ inView_viewOf cfg pre⟩
        · exact Or.inr ⟨hi, _, inView_viewOf cfg pre, hv⟩
    · exact absurd ha act_no_timeoutVote
  · intro v howed
    refine ⟨epochOfHistory cfg pre, lockOf cfg pre, inEpoch_epochOfHistory cfg pre, ?_⟩
    rw [step_output]
    apply discharge_sup
    rcases howed with ⟨hi, hv⟩ | ⟨hi, w, hw, hle⟩
    · simp only [step_input] at hi
      subst hi
      simp [timeoutAnswer, viewOf_of_inView hv]
    · simp only [step_input] at hi
      subst hi
      have : viewOf cfg pre ≤ v := viewOf_of_inView hw ▸ hle
      simp [timeoutAnswer, this]
  · intro p hm
    rcases step_mem hm with h0 | ⟨o, out', _, _, ha⟩
    · obtain ⟨v, hx, _⟩ := mem_timeoutAnswer h0; cases hx
    · obtain ⟨_, _, _, _, _, hj⟩ := act_proposal ha
      exact (sameInputs_step.proposalJustified p).mp (proposalJustifiedB_iff.mp hj)
  · intro r hm
    rcases step_mem hm with h0 | ⟨o, out', _, _, ha⟩
    · obtain ⟨v, hx, _⟩ := mem_timeoutAnswer h0; cases hx
    · obtain ⟨_, _, _, _, _, hj⟩ := act_revote ha
      exact (sameInputs_step.revoteJustified r).mp (revoteJustifiedB_iff.mp hj)

theorem step_global (hg : Global pre) : Global (pre ++ [step cfg leader node pre i]) := by
  have h0 := global_arrive (i := i) hg (out := timeoutAnswer cfg node pre i)
    (fun x hx => by obtain ⟨v, hx, _⟩ := mem_timeoutAnswer hx; exact ⟨_, hx⟩)
  exact discharge_global _ _ h0

/-- **A step leaves nothing owed.** -/
theorem step_settled (o : Obligation) : ¬ Owed cfg leader node (pre ++ [step cfg leader node pre i]) o := by
  by_cases hc : o ∈ candidates cfg (pre ++ [Step.mk i []])
  · rw [show step cfg leader node pre i = Step.mk i _ from rfl]
    exact discharge_settles _ _ [] (by simp) o (Or.inr hc)
  · exact fun ho => hc (mem_candidates sameInputs_grow ho)

end Step

/-! ## Runs -/

variable (cfg leader node)

theorem historyOf_inputs (inputs : Nat → Input) : ∀ n,
    (historyOf cfg leader node inputs n).map (·.input) = (List.range n).map inputs
  | 0 => rfl
  | n + 1 => by
    simp only [historyOf, List.map_append, List.range_succ, historyOf_inputs inputs n]
    rfl

variable {cfg leader node}

/--
**Every history the machine produces obeys the protocol rules**, when every
validity report among its inputs is truthful.
-/
theorem historyOf_protocol {inputs : Nat → Input}
    (hvalid : ∀ n v hash, inputs n = .blockValidated v hash → ∀ b, blockHash b = hash → BlockValid b)
    (n : Nat) : ProtocolHistory cfg leader node (fun _ => True) (fun _ => True) (historyOf cfg leader node inputs n) := by
  suffices ∀ n, ProtocolHistory cfg leader node (fun _ => True) (fun _ => True) (historyOf cfg leader node inputs n)
      ∧ Global (historyOf cfg leader node inputs n) from (this n).1
  intro n
  induction n with
  | zero => exact ⟨protocolHistory_nil, global_nil⟩
  | succ n ih =>
    have hvt : ValidityTruthful (historyOf cfg leader node inputs n ++ [Step.mk (inputs n) []]) := by
      intro v hash hr b hb
      have hr := List.mem_map.symm.mp hr
      rw [List.map_append, historyOf_inputs] at hr
      rcases List.mem_append.mp hr with hr | hr
      · obtain ⟨k, _, hk⟩ := List.mem_map.mp hr
        exact hvalid k v hash hk b hb
      · simp only [List.map_cons, List.map_nil, List.mem_singleton] at hr
        exact hvalid n v hash hr.symm b hb
    exact ⟨protocolHistory_snoc ih.1 (step_ok hvt) (step_global ih.2), step_global ih.2⟩

/-- **After every step of the machine, nothing is owed.** -/
theorem historyOf_settled (inputs : Nat → Input) (n : Nat) (o : Obligation) :
    ¬ Owed cfg leader node (historyOf cfg leader node inputs (n + 1)) o :=
  step_settled o

/-- **A network of machines is prompt**, for every `δ`: no obligation outlives the step that creates it. -/
theorem prompt_of_machine {C : Committee} (N : TimedNetwork cfg leader C) (δ : Nat)
    (hm : ∀ k hk, ∃ inputs, N.trace k hk = traceOf cfg leader k inputs) (hst : ∀ k, C.Honest k → C.Steady k) :
    Prompt N δ := by
  intro k hk n o ho
  obtain ⟨inputs, he⟩ := hm k hk
  rw [he, traceOf_history] at ho
  exact absurd ((owedIn_all (hst k hk)).mp ho) (historyOf_settled inputs n o)

end NewProtocolImpl
