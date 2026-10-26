module

public import NewProtocolImpl.Decide

/-!
# Extending a protocol history by one step

`ProtocolHistory` has two kinds of rule. Most judge one step against the history
up to it, and a step judged once stays judged: appending a step changes no earlier
step's prefix. The rest, at most one vote or proposal per view, are about
everything the history sent. `StepOK` is the first kind for one new step, `Global`
the second for the whole history, and `protocolHistory_snoc` puts them together.
-/

@[expose] public section

namespace NewProtocolImpl

open NewProtocol NewProtocol.Lists History

variable (cfg : Config) (leader : EpochNumber → ViewNumber → Option PubKey) (node : PubKey)

/-- The rules of `ProtocolHistory` that judge step `st` after history `pre`. -/
structure StepOK (pre : History) (st : Step) : Prop where
  vote1 : ∀ vote, Output.send (.vote1 vote) ∈ st.output → vote.signer = node
    ∧ ((∃ sender p vid, (pre ++ [st]).Received (.proposal sender p (some vid))
        ∧ ProposalWellFormed cfg p ∧ BlockValid p ∧ SafeParent p
        ∧ OpensEpochJustified cfg (pre ++ [st]) p ∧ Vote1For vote p)
      ∨ ∃ sender r, (pre ++ [st]).Received (.revote sender r)
        ∧ RevoteWellFormed cfg r ∧ SafeRevote r ∧ Vote1Again vote r)
  vote2 : ∀ vote, Output.send (.vote2 vote) ∈ st.output → vote.signer = node
    ∧ cfg.anchorView < vote.view
    ∧ ∃ c b, (pre ++ [st]).HasCert1 cfg c ∧ (pre ++ [st]).HasProposal cfg b ∧ Certifies c b
      ∧ (pre ++ [st]).HasPayload cfg b.viewNumber b.payloadCommit
      ∧ vote.view = c.view ∧ vote.data = c.data.toVote2
  decide : ∀ blocks c1 c2, Output.decided blocks c1 c2 ∈ st.output →
    ∃ head rest, blocks = head :: rest ∧ (pre ++ [st]).HasCert2 c2 ∧ Commits c2 head
      ∧ (pre ++ [st]).HasCert1 cfg c1 ∧ Certifies c1 head ∧ ChainLinked blocks
      ∧ ∀ b ∈ blocks, (pre ++ [st]).HasProposal cfg b ∧ cfg.anchorView < b.viewNumber
  decideOnce : ∀ i blocks c1 c2, st.output[i]? = some (.decided blocks c1 c2) →
    (blocks.map (·.viewNumber)).Nodup
      ∧ ∀ b ∈ blocks, ¬ (pre ++ [Step.mk st.input (st.output.take i)]).DecidedView b.viewNumber
  vote1Leader : ∀ vote, Output.send (.vote1 vote) ∈ st.output →
    (∃ v, (pre ++ [st]).ViewGround cfg v ∧ vote.view ≤ v)
    ∧ ((∃ sender p vid, (pre ++ [st]).Received (.proposal sender p (some vid))
      ∧ leader p.epoch p.viewNumber = some sender ∧ Vote1For vote p
      ∧ NotBehind cfg (pre ++ [st]) p.epoch)
    ∨ ∃ sender r, (pre ++ [st]).Received (.revote sender r)
      ∧ leader r.cert.data.epoch r.view = some sender ∧ Vote1Again vote r
      ∧ NotBehind cfg (pre ++ [st]) r.cert.data.epoch)
  vote2BeforeTimeout : ∀ vote, Output.send (.vote2 vote) ∈ st.output →
    ¬ (pre ++ [st]).TimedOut vote.view
  timeoutLocked : ∀ vote, Output.send (.timeoutVote vote) ∈ st.output → pre.LockedOn cfg vote.data.lock
  timeoutJustified : ∀ vote, Output.send (.timeoutVote vote) ∈ st.output →
    vote.signer = node ∧ pre.InEpoch cfg vote.data.epoch ∧ pre.HasCert1 cfg vote.data.lock
      ∧ ((st.input = .timeout vote.view ∧ pre.InView cfg vote.view)
        ∨ (st.input = .timeoutOneHonest vote.view ∧ ∃ v, pre.InView cfg v ∧ v ≤ vote.view))
  timeoutAnswered : ∀ v, ((st.input = .timeout v ∧ pre.InView cfg v)
      ∨ (st.input = .timeoutOneHonest v ∧ ∃ w, pre.InView cfg w ∧ w ≤ v)) →
    ∃ e L, pre.InEpoch cfg e ∧ Output.send (.timeoutVote ⟨⟨e, L⟩, v, node⟩) ∈ st.output
  propose : ∀ p, Output.send (.proposal p) ∈ st.output →
    ProposalJustified cfg leader node (pre ++ [st]) p
  revote : ∀ r, Output.send (.revote r) ∈ st.output →
    RevoteJustified cfg leader node (pre ++ [st]) r

/-- The rules of `ProtocolHistory` about everything a history sent, and what the machine keeps with them. -/
structure Global (h : History) : Prop where
  vote1Once : ∀ vote vote', h.Sent (.vote1 vote) → h.Sent (.vote1 vote') →
    vote.data.epoch = vote'.data.epoch → vote.view = vote'.view → vote = vote'
  vote2Once : ∀ vote vote', h.Sent (.vote2 vote) → h.Sent (.vote2 vote') →
    vote.data.epoch = vote'.data.epoch → vote.view = vote'.view → vote = vote'
  proposeOnce : ∀ p p', h.Sent (.proposal p) → h.Sent (.proposal p') →
    p.epoch = p'.epoch → p.viewNumber = p'.viewNumber → p = p'
  revoteOnce : ∀ r, h.Sent (.revote r) →
    (∀ r', h.Sent (.revote r') → r'.cert.data.epoch = r.cert.data.epoch → r'.view = r.view → r' = r)
    ∧ ∀ p, h.Sent (.proposal p) → p.epoch = r.cert.data.epoch → p.viewNumber ≠ r.view

variable {cfg leader node}

theorem sent_snoc {h : History} {st : Step} {m : Message} :
    (h ++ [st]).Sent m ↔ h.Sent m ∨ Output.send m ∈ st.output := by
  simp only [Sent, List.mem_append, List.mem_singleton]
  constructor
  · rintro ⟨s, hs | rfl, hm⟩
    · exact Or.inl ⟨s, hs, hm⟩
    · exact Or.inr hm
  · rintro (⟨s, hs, hm⟩ | hm)
    · exact ⟨s, Or.inl hs, hm⟩
    · exact ⟨st, Or.inr rfl, hm⟩

/-- A step of `h ++ [st]` is an old one with its old prefixes, or `st` after `h`. -/
theorem getElem?_snoc {h : History} {st st' : Step} {n : Nat} (hn : (h ++ [st])[n]? = some st') :
    (h[n]? = some st' ∧ (h ++ [st]).upTo (n + 1) = h.upTo (n + 1) ∧ (h ++ [st]).upTo n = h.upTo n)
      ∨ (st' = st ∧ (h ++ [st]).upTo (n + 1) = h ++ [st] ∧ (h ++ [st]).upTo n = h) := by
  by_cases hlt : n < h.length
  · left
    refine ⟨by rw [List.getElem?_append_left hlt] at hn; exact hn, ?_, ?_⟩
    · simp only [History.upTo]; rw [List.take_append_of_le_length (by omega)]
    · simp only [History.upTo]; rw [List.take_append_of_le_length (by omega)]
  · right
    rw [List.getElem?_append_right (by omega)] at hn
    have hz : n - h.length = 0 := by
      cases hk : n - h.length with
      | zero => rfl
      | succ k => rw [hk] at hn; simp at hn
    rw [hz] at hn
    have hnl : n = h.length := by omega
    subst hnl
    refine ⟨by simpa using hn.symm, ?_, ?_⟩
    · simp only [History.upTo]; exact List.take_of_length_le (by simp)
    · simp only [History.upTo]; simp

/-- What a prefix could lock on, the whole history could lock on. -/
theorem lockable_of_upTo {h : History} {m : Nat} {c : Cert1} (hl : (h.upTo m).Lockable cfg c) :
    h.Lockable cfg c :=
  lockable_mono (fun _ ⟨st, hst, hin⟩ => ⟨st, List.mem_of_mem_take hst, hin⟩) hl

/-- A vote2 a protocol history sent is for a certificate it could lock on. -/
theorem vote2_lockable {h : History} (hp : ProtocolHistory cfg leader node (fun _ => True) (fun _ => True) h) {v2 : Vote2}
    (hs : h.Sent (.vote2 v2)) : ∃ c, h.Lockable cfg c ∧ v2.view = c.view ∧ v2.data = c.data.toVote2 := by
  obtain ⟨st, hst, hm⟩ := hs
  obtain ⟨j, hj⟩ := List.mem_iff_getElem?.mp hst
  obtain ⟨-, -, c, b, hc, hb, hcert, hpay, hv, hd⟩ := hp.vote2Justified j v2 ⟨st, hj, hm⟩ trivial
  exact ⟨c, lockable_of_upTo (Or.inr (Or.inl ⟨hc, b, hb, hcert, hpay⟩)), hv, hd⟩

/-- What `Global` keeps of proposals and re-vote requests is `ProtocolHistory.proposeOnce`. -/
theorem global_once {h : History} (hg : Global h) :
    ∀ m m' v e, h.Sent m → h.Sent m' → m.leaderView = some v → m'.leaderView = some v →
      m.leaderEpoch = some e → m'.leaderEpoch = some e → m = m' := by
  intro m m' v e hs hs' hv hv' he he'
  cases m <;> simp only [Message.leaderView, Message.leaderEpoch, reduceCtorEq] at hv he <;>
    cases m' <;> simp only [Message.leaderView, Message.leaderEpoch, reduceCtorEq, Option.some.injEq] at hv' hv he he'
  · rename_i p p'
    rw [hg.proposeOnce p p' hs hs' (he.trans he'.symm) (hv.trans hv'.symm)]
  · rename_i p r
    exact absurd (hv.trans hv'.symm) ((hg.revoteOnce r hs').2 p hs (he.trans he'.symm))
  · rename_i r p
    exact absurd (hv'.trans hv.symm) ((hg.revoteOnce r hs).2 p hs' (he'.trans he.symm))
  · rename_i r r'
    rw [(hg.revoteOnce r hs).1 r' hs' (he'.trans he.symm) (hv'.trans hv.symm)]

/-- **Extending a protocol history by a step that obeys the rules.** -/
theorem protocolHistory_snoc {h : History} {st : Step} (hp : ProtocolHistory cfg leader node (fun _ => True) (fun _ => True) h)
    (hs : StepOK cfg leader node h st) (hg : Global (h ++ [st])) :
    ProtocolHistory cfg leader node (fun _ => True) (fun _ => True) (h ++ [st]) := by
  refine ⟨⟨?_, fun v v' a b _ he hv => hg.vote1Once v v' a b he hv, ?_,
    fun v v' a b _ he hv => hg.vote2Once v v' a b he hv, ?_, ?_, ?_, ?_⟩, ?_, ?_, ?_, ?_, ?_,
    fun m m' v e hs hs' hv hv' he he' _ => global_once hg m m' v e hs hs' hv hv' he he'⟩
  · rintro n vote ⟨st', hn, hm⟩ _
    rcases getElem?_snoc hn with ⟨hold, hu, _⟩ | ⟨rfl, hu, _⟩
    · rw [hu]; exact hp.vote1Justified n vote ⟨st', hold, hm⟩ trivial
    · rw [hu]; exact hs.vote1 vote hm
  · rintro n vote ⟨st', hn, hm⟩ _
    rcases getElem?_snoc hn with ⟨hold, hu, _⟩ | ⟨rfl, hu, _⟩
    · rw [hu]; exact hp.vote2Justified n vote ⟨st', hold, hm⟩ trivial
    · rw [hu]; exact hs.vote2 vote hm
  · rintro n vote ⟨st', hn, hm⟩ _ tv htv _
    rcases getElem?_snoc hn with ⟨hold, hu, _⟩ | ⟨rfl, hu, _⟩
    · rw [hu] at htv; exact hp.vote2BeforeTimeout n vote ⟨st', hold, hm⟩ trivial tv htv trivial
    · rw [hu] at htv
      exact Nat.lt_of_not_le fun hle => hs.vote2BeforeTimeout vote hm ⟨tv, htv, hle⟩
  · rintro n vote ⟨st', hn, hm⟩ _
    rcases getElem?_snoc hn with ⟨hold, _, hu⟩ | ⟨rfl, _, hu⟩
    · rw [hu]; exact hp.timeoutLock n vote ⟨st', hold, hm⟩ trivial
    · rw [hu]
      intro v2 hs2 _
      obtain ⟨c, hc, hv, hd⟩ := vote2_lockable hp hs2
      rw [hv, hd]; exact (hs.timeoutLocked vote hm).2 c hc
  · intro n st' blocks c1 c2 hn hm _
    rcases getElem?_snoc hn with ⟨hold, hu, _⟩ | ⟨rfl, hu, _⟩
    · rw [hu]; exact hp.decideJustified n st' blocks c1 c2 hold hm trivial
    · rw [hu]; exact hs.decide blocks c1 c2 hm
  · intro n st' j blocks c1 c2 hn hj _
    rcases getElem?_snoc hn with ⟨hold, _, hu⟩ | ⟨rfl, _, hu⟩
    · rw [hu]; exact hp.decideOnce n st' j blocks c1 c2 hold hj trivial
    · rw [hu]; exact hs.decideOnce j blocks c1 c2 hj
  · rintro n vote ⟨st', hn, hm⟩ _
    rcases getElem?_snoc hn with ⟨hold, hu, _⟩ | ⟨rfl, hu, _⟩
    · rw [hu]; exact hp.vote1Leader n vote ⟨st', hold, hm⟩ trivial
    · rw [hu]; exact hs.vote1Leader vote hm
  · intro n st' vote hn hm _
    rcases getElem?_snoc hn with ⟨hold, _, hu⟩ | ⟨rfl, _, hu⟩
    · rw [hu]; exact hp.timeoutJustified n st' vote hold hm trivial
    · rw [hu]; exact hs.timeoutJustified vote hm
  · intro n st' v hn howed _
    rcases getElem?_snoc hn with ⟨hold, _, hu⟩ | ⟨rfl, _, hu⟩
    · rw [hu] at howed ⊢; exact hp.timeoutAnswered n st' v hold howed fun _ _ => ⟨trivial, trivial⟩
    · rw [hu] at howed ⊢; exact hs.timeoutAnswered v howed
  · rintro n p ⟨st', hn, hm⟩ _
    rcases getElem?_snoc hn with ⟨hold, hu, _⟩ | ⟨rfl, hu, _⟩
    · rw [hu]; exact hp.proposeJustified n p ⟨st', hold, hm⟩ trivial
    · rw [hu]; exact hs.propose p hm
  · rintro n r ⟨st', hn, hm⟩ _
    rcases getElem?_snoc hn with ⟨hold, hu, _⟩ | ⟨rfl, hu, _⟩
    · rw [hu]; exact hp.revoteJustified n r ⟨st', hold, hm⟩ trivial
    · rw [hu]; exact hs.revote r hm

/-- The empty history obeys every rule. -/
theorem protocolHistory_nil : ProtocolHistory cfg leader node (fun _ => True) (fun _ => True) [] := by
  refine ⟨⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> simp [Sent, SentAt]

theorem global_nil : Global [] := by
  refine ⟨?_, ?_, ?_, ?_⟩ <;> simp [Sent]

end NewProtocolImpl
