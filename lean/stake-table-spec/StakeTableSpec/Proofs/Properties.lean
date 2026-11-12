module

public import StakeTableSpec.Spec
public import StakeTableSpec.Proofs.Table
public import StakeTableSpec.Proofs.Select
public import StakeTableSpec.Proofs.Events
public import StakeTableSpec.Proofs.Reward
public import StakeTableSpec.Proofs.Schedule
public import StakeTableSpec.Proofs.Reads
public import StakeTableSpec.Proofs.Payout
public import StakeTableSpec.Proofs.Commitment

/-!
# The claims, proved

One theorem for each claim of `StakeTableSpec.Spec`, from the lemmas of the other
proof modules.
-/

@[expose] public section

namespace StakeTable

theorem delegationsNonNegative : DelegationsNonNegative := fun _ d a hv => delegated_nonneg hv d a

theorem stakeIsDelegations : StakeIsDelegations := fun h a =>
  stake_eq_sum h a (nodup_dedup _) fun _ hd => mem_dedup.2 hd

theorem stakeBounded : StakeBounded := fun _ a hv => stake_bounded hv a

theorem tableDistinct : TableDistinct := fun _ hv => table_distinct hv

theorem exitIsFinal : ExitIsFinal := fun _ _ _ hv he hc => not_accepts_of_exited hv he hc

theorem keysAdmittedOnce : KeysAdmittedOnce := fun _ hv => keys_admitted_once hv

theorem commitmentFixesCommittee : CommitmentFixesCommittee := fun _ _ _ _ _ hv hv' hs hr hr' =>
  commitment_fixes_committee hv hv' hs hr hr'

theorem readsUnique : ReadsUnique := fun _ _ _ hr hr' =>
  ⟨reads_unique hr hr', reads_valid hr⟩

theorem l1OrderFunctional : L1OrderFunctional := fun _ _ hk =>
  let ⟨o, ho⟩ := inL1Order_exists hk
  ⟨o, ho, fun _ ho' => inL1Order_unique ho' ho⟩

theorem resumeIsL1Order : ResumeIsL1Order := fun _ _ _ _ _ _ hk hf => resume_inL1Order hk hf

theorem chunkedReadsAll : ChunkedReadsAll := fun _ logs _ _ _ _ hc => chunked_reads_all logs hc

theorem activeSetFunctional : ActiveSetFunctional := fun usable _ hidx =>
  let ⟨r, hr⟩ := isActiveSet_exists usable hidx
  ⟨r, hr, fun _ hr' => isActiveSet_unique hidx hr' hr⟩

theorem activeSetFull : ActiveSetFull := fun _ _ _ h => ⟨activeSet_ne_nil h, activeSet_length h⟩

theorem activeSetByStake : ActiveSetByStake := fun _ _ _ h _ hv _ hw hout =>
  activeSet_stake h hv hw hout

theorem activeSetStakeRatio : ActiveSetStakeRatio := fun _ _ _ h _ hv _ hw =>
  activeSet_stake_ratio h hv hw

theorem committeeStakeWithinTable : CommitteeStakeWithinTable := fun _ _ _ ⟨_, h⟩ =>
  activeSet_stake_le h

theorem activeSetDistinct : ActiveSetDistinct := fun _ _ _ ⟨hv, h⟩ =>
  activeSet_distinct (table_distinct hv) h

theorem committeeUnique : CommitteeUnique := fun _ _ _ _ _ h h' => committeeAt_unique h h'

theorem scheduleOrderFunctional : ScheduleOrderFunctional := fun drb _ hk =>
  ⟨_, inScheduleOrder_mergeSort drb hk,
    fun _ ho' => inScheduleOrder_unique ho' (inScheduleOrder_mergeSort drb hk)⟩

/-- What the schedule needs of a committee: distinct keys, each member holding one, and stake. -/
theorem committee_schedulable {usable : NetAddr → Bool} {h : History}
    {as : List Member} (hc : IsCommittee usable h as) :
    Distinct as ∧ (∀ m ∈ as, m.blsKey.isSome) ∧ 0 < (as.map (·.stake)).sum := by
  obtain ⟨hv, ha⟩ := hc
  have hm : ∀ m ∈ as, m.blsKey.isSome ∧ 0 < m.stake := fun m hm => by
    have := (activeSet_eligible ha hm).1
    simp only [Member.candidate, Bool.and_eq_true, decide_eq_true_eq] at this
    exact ⟨this.1.1.1.1.2, this.2⟩
  refine ⟨activeSet_distinct (table_distinct hv) ha, fun m hm' => (hm m hm').1, ?_⟩
  obtain ⟨m, hm'⟩ := List.exists_mem_of_ne_nil _ (activeSet_ne_nil ha)
  have := (hm m hm').2
  have := le_totalStake (List.mem_map_of_mem (f := fun m : Member => (⟨[], m.stake⟩ : Entry)) hm')
  simp only [totalStake, List.map_map] at this
  exact Nat.lt_of_lt_of_le ‹0 < m.stake› this

theorem leaderFunctional : LeaderFunctional := fun _ _ _ b v hc =>
  let ⟨hd, hk, hs⟩ := committee_schedulable hc
  let ⟨m, hm⟩ := leads_exists b v hd hk hs
  ⟨m, hm, fun _ hm' => leads_unique hd hk hm' hm⟩

theorem leaderShareIsStake : LeaderShareIsStake := fun _ _ _ b _ hc hm =>
  let ⟨hd, hk, _⟩ := committee_schedulable hc
  leads_iff b hd hk hm

theorem leaderIgnoresListing : LeaderIgnoresListing := fun _ _ _ _ _ hp =>
  ⟨leads_perm hp, leads_perm hp.symm⟩

theorem rewardFunctional : RewardFunctional := fun _ _ _ hd =>
  let ⟨r, hr⟩ := blockReward_exists hd
  ⟨r, hr, fun _ hr' => blockReward_unique hr' hr⟩

theorem issuanceBounded : IssuanceBounded := fun _ _ _ _ h => issuance_bounded h

theorem rewardMonotone : RewardMonotone := fun _ _ _ _ _ _ hs h h' => blockReward_mono hs h h'

theorem payoutAddsUp : PayoutAddsUp := fun _ a r hv => payout_adds_up hv a r

theorem commissionKept : CommissionKept := fun _ _ r hv hS hc => commission_kept hv r hS hc

theorem commitmentFixesPayout : CommitmentFixesPayout := fun _ _ _ r x hs ha =>
  commitment_fixes_payout hs ha r x

end StakeTable
