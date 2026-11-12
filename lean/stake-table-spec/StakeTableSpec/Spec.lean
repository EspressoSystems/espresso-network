module

public import StakeTableSpec.Basic

/-!
# The stake table specification

## What a stake table is

Espresso's validators register on L1, in the stake table contract, and delegators
delegate stake to them there. The stake table is what the contract's events say
about the validators at some point: who is registered, with which keys, network
address and commission, and how much stake each holds from which delegators. From it
the network takes each epoch's committee, who leads each view of the epoch, and who
is paid the block reward.

## How it is represented

The specification does not keep a table and update it. It keeps the history of
changes the table took (`StakeTable.History`), and reads everything off it: the
registered validators are those that registered and did not exit, a validator's keys
are those the last change that sets them gave it, and its stake is the sum of what
was delegated to it minus what was withdrawn (`StakeTable.table`). There are no
update rules to get wrong; each fact is a filter, a sum or a last value.

What the history may hold is the one rule: a change is taken only when it makes sense
after the changes before it (`StakeTable.Accepts`), and a history of taken changes is
valid (`StakeTable.Valid`). The contract's events come to these changes, each event
to at most one (`StakeTable.changeOf`), and a list of events makes a history by
taking each change the table takes and dropping the others (`StakeTable.Reads`). The
events are read in the order they have on L1 (`StakeTable.InL1Order`).

From the table follow:

* an epoch's committee, the validators with the most stake among those allowed
  (`StakeTable.IsActiveSet`), read from L1 up to the epoch's root
  (`StakeTable.CommitteeAt`);
* the leader of each view, drawn from the committee in proportion to stake
  (`StakeTable.Leads`);
* the block reward, from the share of the supply that is staked
  (`StakeTable.BlockReward`), and how a leader's validator and delegators share it
  (`StakeTable.credited`).

## What is claimed

The last section states the claims, which fall into the groups below.

The table is well formed: no delegation goes negative, stake is the sum of the
delegations and fits in 256 bits, no two members share an account or a key, no key
is admitted twice, and nothing about a validator that exited is taken again.

Everything is determined: events make at most one history, none if the node stops
on one of them; logs have exactly one L1 order, a resumed read gives the same
order as a full one, and a read in chunks loses and repeats no log; a stake table
with distinct accounts, as every valid one has, has exactly one committee or none,
and the logs on L1 up to a block at most one committee; each view of a committee
has exactly one leader, whatever order the committee is listed in; and the inputs
exactly one block reward. Every node that reads the same L1 logs therefore agrees
on all of it.

A node that does not read L1 itself, as the light client, takes a stake table built
from events a server sends and checks it against the commitment in an epoch root
header. The commitment leaves out the order validators registered in, so the server
can choose that order. The committee does not depend on it: tables with the same
commitment select the same committee, listed in the same order
(`StakeTable.CommitmentFixesCommittee`), and pay the same rewards
(`StakeTable.CommitmentFixesPayout`).

It is fair: no eligible validator left out of a committee has more stake than a
member, a member leads views in proportion to its stake, a payout adds up to the
reward, and a validator keeps at least its commission and less than one unit more
per delegator.

It is bounded: the stakes of a committee are within a factor of
`vidTargetTotalStake` of each other, a committee holds no more stake than its table,
the block rewards of a year are at most 2.13% of the supply, and more stake never
lowers the block reward.

## What is not modelled

Whether a key decodes and whether a signature verifies are decided before a change is
made of an event, and whether a network address is usable before the committee is
selected; all are taken as given. So are the hash that draws a view's leader, which
header is an epoch's root, and which logs L1 holds.

The rules are those of protocol version 0.7. Earlier versions broke ties in stake by
registration order, listed the committee in that order, and before 0.6 did not ask
for a network key and address; none of that is modelled.

The data is in `StakeTableSpec.Types`, and the helpers the definitions are written
with in `StakeTableSpec.Basic`. That the node computes all of this is the
implementation's part (`lean/stake-table-impl`), checked there by proof and against
the node by differential tests.
-/

@[expose] public section

namespace StakeTable

/-!
## The stake table

A change is about one validator (`StakeTable.Change.validator`). The table holds the
validators that registered and did not exit, in the order they registered
(`StakeTable.registered`). A validator's keys, address and commission are those the
last change that sets them gave it, and its stake is what was delegated to it minus
what was withdrawn (`StakeTable.stake`); likewise for each delegator
(`StakeTable.delegated`). A key, once held, is never admitted again
(`StakeTable.usedBls`).

A change is taken when it makes sense for the table so far (`StakeTable.Accepts`): a
new validator registers with keys nobody held, a registered validator is the one
changed, and a delegator withdraws no more than it delegated. A history is valid if
every change in it was taken (`StakeTable.Valid`).
-/

/-- The validators that registered, in the order they did. -/
def registrations (h : History) : List Address :=
  h.filterMap fun | .register a .. => some a | _ => none

/-- Validator `a` exited. -/
def Exited (h : History) (a : Address) : Prop := Change.exit a ∈ h

instance (h : History) (a : Address) : Decidable (Exited h a) := by
  unfold Exited; infer_instance

/-- The validators registered and not exited, in the order they registered. -/
def registered (h : History) : List Address :=
  (registrations h).filter fun a => !decide (Exited h a)

/-- Validator `a` is registered: it registered and did not exit. -/
def Registered (h : History) (a : Address) : Prop := a ∈ registered h

/-- What delegator `d` has delegated to validator `a`, net of withdrawals. -/
def delegated (h : History) (d a : Address) : Int := (h.map (·.flow d a)).sum

/-- What is delegated to validator `a`, net of withdrawals. -/
def stake (h : History) (a : Address) : Int := (h.map (·.inflow a)).sum

/-- The delegators of `a`, each once: those that ever delegated to it or withdrew from it. -/
def delegators (h : History) (a : Address) : List Address :=
  dedup (delegatorsOf h a)

/-- The consensus keys ever admitted. -/
def usedBls (h : History) : List BlsKey := h.flatMap (·.blsKeys)

/-- The light client keys ever admitted. -/
def usedSchnorr (h : History) : List SchnorrKey := h.flatMap (·.schnorrKeys)

/-- The network keys ever admitted. -/
def usedX25519 (h : History) : List X25519Key := h.flatMap (·.x25519Keys)

/-- Validator `a` as the table shows it. -/
def member (h : History) (a : Address) : Member := {
    account := a,
    blsKey := (latest (·.blsKeyOf a) h).join,
    schnorrKey := (latest (·.schnorrKeyOf a) h).join,
    x25519Key := (latest (·.x25519KeyOf a) h).join,
    p2pAddr := (latest (·.p2pAddrOf a) h).join,
    stake := (stake h a).toNat,
    commission := (latest (·.commissionOf a) h).getD 0,
    authenticated := (latest (·.authenticatedOf a) h).getD false
  }

/-- The stake table: its members, in the order they registered. -/
def table (h : History) : List Member := (registered h).map (member h)

/--
The stake table takes change `c` after history `h`.

A registration needs an account that never registered, keys nobody held, and both
consensus keys if it is authenticated. A delegation needs some amount, and keeps the
stake within 256 bits. A withdrawal needs a delegation, and takes no more than it.
Every other change needs its validator registered, a key it sets nobody held, and a
commission at most `maxCommission`.
-/
def Accepts (h : History) : Change → Prop
  | .register a bls schnorr x25519 _ _ auth => a ∉ registrations h
      ∧ (∀ k ∈ bls, k ∉ usedBls h)
      ∧ (∀ k ∈ schnorr, k ∉ usedSchnorr h)
      ∧ (∀ k ∈ x25519, k ∉ usedX25519 h)
      ∧ (auth = true → bls.isSome ∧ schnorr.isSome)

  | .exit a => Registered h a

  | .delegate _ a n => Registered h a
      ∧ 0 < n
      ∧ stake h a + n < U256

  | .undelegate d a n => Registered h a
      ∧ 0 < delegated h d a
      ∧ n ≤ delegated h d a

  | .rekey a bls schnorr => Registered h a
      ∧ bls ∉ usedBls h
      ∧ schnorr ∉ usedSchnorr h

  | .setCommission a c => Registered h a ∧ c ≤ maxCommission

  | .setX25519 a k => Registered h a ∧ ∀ k' ∈ k, k' ∉ usedX25519 h

  | .setP2pAddr a _ => Registered h a

/-- A history of changes each taken after those before it. -/
inductive Valid : History → Prop where
  /-- Nothing happened yet. -/
  | nil : Valid []

  /-- A change taken after a valid history. -/
  | snoc {h : History} {c : Change} : Valid h → Accepts h c → Valid (h ++ [c])

/--
Members have distinct accounts, and no two hold the same consensus, light client or
network key.
-/
structure Distinct (ms : List Member) : Prop where
  /-- No account twice. -/
  accounts : (ms.map (·.account)).Nodup

  /-- No consensus key shared. -/
  bls : ∀ m ∈ ms, ∀ m' ∈ ms, ∀ k, m.blsKey = some k → m'.blsKey = some k → m = m'

  /-- No light client key shared. -/
  schnorr : ∀ m ∈ ms, ∀ m' ∈ ms, ∀ k, m.schnorrKey = some k → m'.schnorrKey = some k → m = m'

  /-- No network key shared. -/
  x25519 : ∀ m ∈ ms, ∀ m' ∈ ms, ∀ k, m.x25519Key = some k → m'.x25519Key = some k → m = m'

/--
`h` and `h'` agree on everything the node's stake table commitment covers: the
registered validators, each as a member with its delegations, the keys ever admitted,
and the validators that exited.

The commitment takes each of these as a set, sorted by address or key, so the order
the validators registered in is not covered. It is a hash of this data, taken to be
collision free: equal commitments mean histories that agree in this sense.
-/
def SameCommitment (h h' : History) : Prop :=
  (∀ a, Registered h a ↔ Registered h' a)
    ∧ (∀ a, Registered h a → member h a = member h' a ∧ ∀ d, delegated h d a = delegated h' d a)
    ∧ (∀ k, k ∈ usedBls h ↔ k ∈ usedBls h')
    ∧ (∀ k, k ∈ usedSchnorr h ↔ k ∈ usedSchnorr h')
    ∧ (∀ k, k ∈ usedX25519 h ↔ k ∈ usedX25519 h')
    ∧ (∀ a, Exited h a ↔ Exited h' a)

/-!
## From events to changes

The node reads the contract's events (`StakeTable.Event`) with keys decoded and
signatures checked. Each event comes to at most one change (`StakeTable.changeOf`):

* a registration emitted by `StakeTable` is authenticated if both its
  consensus keys decoded, its signatures not being checked;
* a later registration is authenticated if both signatures verified, except that a
  light client key admitted before is dropped, and the registration is then not
  authenticated;
* a key update comes to a change only if both keys decoded and, from
  `StakeTableV2` on, both signatures verified;
* every other event comes to the change of the same name.

The events make a history (`StakeTable.Reads`): in turn, each event's change joins
it if the table takes the change. The node passes over a key update whose keys did
not decode, and a `StakeTable` registration or a key update whose only fault is a
light client key admitted before (`StakeTable.Passes`); on any other event it stops,
and the events make no history.
-/

/-- The change an event comes to after history `h`, if any. -/
def changeOf (h : History) : Event → Option Change
  | .register a bls schnorr c => some (.register a bls schnorr none none c (bls.isSome && schnorr.isSome))
  | .registerV2 a keys c => some (registration h a keys c none none)
  | .registerV3 a keys c x p => some (registration h a keys c x p)
  | .deregister a => some (.exit a)
  | .delegate d a n => some (.delegate d a n)
  | .undelegate d a n => some (.undelegate d a n)
  | .keyUpdate a (some b) (some k) => some (.rekey a b k)
  | .keyUpdate .. => none
  | .keyUpdateV2 a (.verified b k) => some (.rekey a b k)
  | .keyUpdateV2 .. => none
  | .commissionUpdate a c => some (.setCommission a c)
  | .x25519Update a k => some (.setX25519 a k)
  | .p2pAddrUpdate a p => some (.setP2pAddr a p)
where
  /--
  The change a registration emitted by `StakeTableV2` or later comes to: without a
  light client key admitted before, and then not authenticated.
  -/
  registration (h : History) (a : Address) (keys : RegistrationKeys) (commission : Nat)
      (x25519 : Option X25519Key) (p2pAddr : Option NetAddr) : Change :=
    if keys.schnorr.any (· ∈ usedSchnorr h)
      then .register a keys.bls none x25519 p2pAddr commission false
      else .register a keys.bls keys.schnorr x25519 p2pAddr commission keys.isVerified

/--
The node passes over event `e` after history `h` and reads on: a key update whose keys
did not decode, or a `StakeTable` registration or a key update that the table would
take but for a light client key admitted before.

A key update emitted by `StakeTableV2` or later whose signatures do not verify is not
passed over: the node stops on it.
-/
def Passes (h : History) : Event → Prop
  | .register a bls (some k) c =>
    k ∈ usedSchnorr h ∧ Accepts h (.register a bls none none none c false)
  | .keyUpdate a (some b) (some k) | .keyUpdateV2 a (.verified b k) =>
    k ∈ usedSchnorr h ∧ Registered h a ∧ b ∉ usedBls h
  | .keyUpdate .. => True
  | .keyUpdateV2 _ .invalidBls | .keyUpdateV2 _ .invalidSchnorr => True
  | _ => False

/-- Events `es`, in order, make history `h`. -/
inductive Reads : List Event → History → Prop where
  /-- No events make the empty history. -/
  | nil : Reads [] []
  /-- An event whose change the table takes adds it. -/
  | take {es : List Event} {h : History} {e : Event} {c : Change} :
      Reads es h → changeOf h e = some c → Accepts h c → Reads (es ++ [e]) (h ++ [c])
  /-- An event the node passes over adds nothing. -/
  | drop {es : List Event} {h : History} {e : Event} :
      Reads es h → Passes h e → Reads (es ++ [e]) h

/-!
## Reading events from L1

A log's place on L1 is its block number and its index within the block
(`StakeTable.LogKey`), and no two logs share one. The node replays the logs in that
order (`StakeTable.InL1Order`). It reads them in chunks of blocks
(`StakeTable.Chunked`), and it keeps what it has read: on a later read it fetches only
the blocks after the last one it kept, appends what it fetched to what it kept, and
drops a log that directly repeats the one before it (`StakeTable.resume`).

Which logs exist, and that every read returns all of a range's logs, is the L1
client's part, not modelled here.
-/

/--
`out` is `logs` in L1 order: the same logs, each before every one after it.

The logs carry any payload `α`; only their keys are read.
-/
def InL1Order {α : Type} (logs out : List (LogKey × α)) : Prop :=
  out.Perm logs ∧ out.Pairwise fun a b => a.1.before b.1 = true

/--
The block ranges a read is split into: `[first, last]` in order, each range at most
`size` blocks, both ends inclusive.
-/
def Chunked (first last size : Nat) (chunks : List (Nat × Nat)) : Prop :=
  chunks.flatMap (fun c => List.range' c.1 (c.2 + 1 - c.1)) = List.range' first (last + 1 - first)
    ∧ ∀ c ∈ chunks, c.1 ≤ c.2 ∧ c.2 + 1 - c.1 ≤ size

/--
A read that resumes from what was kept: the kept events, then the fetched ones,
without directly repeated logs.
-/
def resume {α : Type} [DecidableEq α] (kept fetched : List (LogKey × α)) : List (LogKey × α) :=
  dedupAdjacent (kept ++ fetched)

/-!
## The committee

Which members of the stake table form an epoch's committee (`StakeTable.IsActiveSet`).

A validator is a candidate if it is authenticated, holds both consensus keys and a
network key, has a usable network address and has stake. A candidate is eligible if
it has at least the largest candidate stake divided by `vidTargetTotalStake`. The
active validators are the `maxValidators` eligible ones that rank first, or all of
them if there are fewer: more stake ranks first, and of equal stakes the smaller
account (`StakeTable.RanksBefore`). They are listed by account
(`StakeTable.ByAccount`); certificates name their signers by position in this list.
With no candidate there is no committee.

Neither the ranking nor the listing reads the order the validators registered in, so
the committee depends only on what the stake table commitment covers.

An epoch's stake table is read from L1 up to the block its epoch root header records
as finalized (`StakeTable.CommitteeAt`). Which header is an epoch's root is
consensus's part, and whether a network address is usable is decided outside this
model, as whether a key decodes is: the definitions take both as given.
-/

/-- The most validators active at once. The node's `MAX_VALIDATORS`. -/
def maxValidators : Nat := 100

/--
The divisor of the minimum stake. The node's `VID_TARGET_TOTAL_STAKE`.

An eligible candidate has at least the largest candidate stake divided by this.
-/
def vidTargetTotalStake : Nat := 1000

/-- Whether a validator may be active, with `usable` the network addresses that are usable. -/
def Member.candidate (usable : NetAddr → Bool) (v : Member) : Bool :=
  v.authenticated && v.blsKey.isSome && v.schnorrKey.isSome && v.x25519Key.isSome
    && v.p2pAddr.any usable && decide (0 < v.stake)

/-- The candidates among `vs`. -/
def candidates (usable : NetAddr → Bool) (vs : List Member) : List Member :=
  vs.filter (·.candidate usable)

/-- The largest stake in a list of validators, zero if it is empty. -/
def maxStake (vs : List Member) : Nat :=
  vs.foldr (fun v m => max v.stake m) 0

/-- The candidates with at least the minimum stake. -/
def eligible (cs : List Member) : List Member :=
  cs.filter fun v => decide (maxStake cs / vidTargetTotalStake ≤ v.stake)

/-- `p` ranks before `q`: it has more stake, or as much and a smaller account. -/
def RanksBefore (p q : Member) : Bool :=
  decide (q.stake < p.stake)
    || (decide (p.stake = q.stake) && decide (p.account.toNat < q.account.toNat))

/--
`S` is the top `n` of the validators `ts`: as many of them as `n` allows, each once,
and whoever ranks before a member is a member.
-/
structure Top (n : Nat) (ts S : List Member) : Prop where
  /-- No member twice. -/
  nodup : S.Nodup
  /-- The members are drawn from `ts`. -/
  subset : ∀ m ∈ S, m ∈ ts
  /-- `n` of them, or all of `ts` if there are fewer. -/
  size : S.length = min n ts.length
  /-- Whoever ranks before a member is a member. -/
  closed : ∀ q ∈ S, ∀ p ∈ ts, RanksBefore p q = true → p ∈ S

/-- `as` is listed by account: each account is smaller than those after it. -/
def ByAccount (as : List Member) : Prop :=
  as.Pairwise fun a b => a.account.toNat < b.account.toNat

/--
`r` is the committee of `vs`: none if there is no candidate, otherwise the top
`maxValidators` of the eligible ones, listed by account.
-/
def IsActiveSet (usable : NetAddr → Bool) (vs : List Member) : Option (List Member) → Prop
  | none => candidates usable vs = []
  | some as => candidates usable vs ≠ []
      ∧ Top maxValidators (eligible (candidates usable vs)) as ∧ ByAccount as

/-- `as` is the committee of the table of a valid history `h`. -/
def IsCommittee (usable : NetAddr → Bool) (h : History) (as : List Member) : Prop :=
  Valid h ∧ IsActiveSet usable (table h) (some as)

/--
`r` is the committee of the stake table read from `logs`, the contract's logs on L1,
up to block `B`: the logs up to `B`, in L1 order, make a history, and `r` is the
committee of its table.
-/
def CommitteeAt (usable : NetAddr → Bool) (logs : List (LogKey × Event)) (B : Nat)
    (r : Option (List Member)) : Prop :=
  ∃ out h, InL1Order (logs.filter fun l => decide (l.1.1 ≤ B)) out ∧ Reads (out.map (·.2)) h
    ∧ IsActiveSet usable (table h) r

/-!
## The leader schedule

Who leads each view of an epoch, from its committee and its random beacon
(`StakeTable.Leads`).

The members are put in the order of their consensus keys `xor`ed with the DRB result
(`StakeTable.InScheduleOrder`). Their stakes, laid end to end in that order, split
`[0, total stake)` into one share each (`StakeTable.ShareContains`). Each view draws
a breakpoint, and its leader is the member whose share contains the breakpoint
modulo the total stake (`StakeTable.Draws`).

The order compares keys as bytes, and the breakpoint is a hash of the DRB result,
the view and the keys in schedule order. Neither the encoding of keys nor the hash is
modelled: both are given with the DRB result (`StakeTable.Beacon`), the encoding
taken to be injective.
-/

/-- `out` is `entries` in schedule order: their `xor`ed keys increase, lexicographically. -/
def InScheduleOrder (drb : List Nat) (entries out : List Entry) : Prop :=
  out.Perm entries ∧ out.Pairwise fun a b => xorKey drb a.key < xorKey drb b.key

/--
The share of the entry at position `i` contains breakpoint `b`: `b` is at least the
stake before it and below the stake up to and including it.
-/
def ShareContains (es : List Entry) (b i : Nat) : Prop :=
  i < es.length ∧ stakeBefore es i ≤ b ∧ b < stakeBefore es (i + 1)

instance (es : List Entry) (b i : Nat) : Decidable (ShareContains es b i) := by
  unfold ShareContains; infer_instance

/-- `e` holds the share of `es` that contains breakpoint `b` modulo the total stake. -/
def Draws (es : List Entry) (b : Nat) (e : Entry) : Prop :=
  0 < totalStake es ∧ ∃ i, ShareContains es (b % totalStake es) i ∧ es[i]? = some e

/-- What an epoch's schedule draws on beside its committee. -/
structure Beacon where
  /-- The epoch's DRB result. -/
  drb : List Nat
  /-- A consensus key as bytes. -/
  keyBytes : BlsKey → List Nat
  /-- Distinct keys have distinct bytes. -/
  keyBytes_injective : Function.Injective keyBytes
  /--
  The breakpoint of a view, from the DRB result, the view and the consensus keys in
  schedule order. The node takes it from a SHA-512 hash of them.
  -/
  hash : List Nat → Nat → List (List Nat) → Nat

/-- A member as the schedule sees it: its consensus key as bytes, and its stake. -/
def Beacon.entry (b : Beacon) (m : Member) : Entry :=
  ⟨(m.blsKey.map b.keyBytes).getD [], m.stake⟩

/--
`m` leads view `v` of committee `as`: with the committee in schedule order, `m`'s
share contains the breakpoint the view draws.
-/
def Leads (b : Beacon) (as : List Member) (v : Nat) (m : Member) : Prop :=
  m ∈ as ∧ ∃ o, InScheduleOrder b.drb (as.map b.entry) o
    ∧ Draws o (b.hash b.drb v (o.map (·.key))) (b.entry m)

/-!
## The block reward

The reward each block of an epoch pays, from the token supply, the stake of the
active validators, and the average block time (`StakeTable.BlockReward`).

With `p` the share of the supply that is staked, the yearly reward rate is
`0.03 / sqrt(2 * max(p, 0.01))`, and the yearly inflation `p` times that. Spread
over the blocks of a year, the reward of one block is

    supply * p * 0.03 / sqrt(2 * max(p, 0.01)) / blocksPerYear

rounded to the nearest integer, a half to the even one. The value is the square root
of a fraction, so it is stated through its square (`StakeTable.rewardSquare`).

The node computes it in decimal arithmetic to 100 significant digits, which agrees
with the exact value except within about `10⁻⁷⁹` of a half-integer.
-/

/-- Milliseconds in a year of 365 days. The node's `MILLISECONDS_PER_YEAR`. -/
def msPerYear : Nat := 86400000 * 365

/-- Blocks in a year at an average block time of `avgMs` milliseconds, rounded down. -/
def blocksPerYear (avgMs : Nat) : Nat := msPerYear / avgMs

/--
The inputs give a reward: some supply, no more stake than supply, and at least one
block a year. Otherwise the node reports an error.
-/
def RewardDefined (supply stake avgMs : Nat) : Prop :=
  0 < supply ∧ stake ≤ supply ∧ 0 < avgMs ∧ 0 < blocksPerYear avgMs

/--
The square of the exact block reward, as a numerator and a denominator, with
`blocks` blocks a year.

With at least a hundredth of the supply staked it is
`(0.03 * stake / blocks)² / (2 * stake / supply)`, otherwise
`(0.03 * stake / blocks)² / 0.02`.
-/
def rewardSquare (supply stake blocks : Nat) : Nat × Nat :=
  if supply ≤ 100 * stake
    then (9 * stake * supply, 20000 * blocks * blocks)
    else (9 * stake * stake, 200 * blocks * blocks)

/--
`r` is the integer nearest the square root of `num / den`, a half rounding to the even
neighbour, as the node's decimal rounding does.
-/
def NearestRoot (num den r : Nat) : Prop :=
  0 < den
    ∧ (2 * r - 1) * (2 * r - 1) * den ≤ 4 * num ∧ 4 * num ≤ (2 * r + 1) * (2 * r + 1) * den
    ∧ ((2 * r - 1) * (2 * r - 1) * den = 4 * num → r % 2 = 0)
    ∧ (4 * num = (2 * r + 1) * (2 * r + 1) * den → r % 2 = 0)

/-- `r` is the block reward for this supply, stake and average block time. -/
def BlockReward (supply stake avgMs r : Nat) : Prop :=
  RewardDefined supply stake avgMs
    ∧ NearestRoot (rewardSquare supply stake (blocksPerYear avgMs)).1
        (rewardSquare supply stake (blocksPerYear avgMs)).2 r

/-!
## Paying the block reward

The reward of a block goes to its leader's validator and delegators
(`StakeTable.credited`). Each delegator is paid its share of the stake times what the
commission leaves, rounded down (`StakeTable.delegatorPaid`), and the validator
keeps the rest (`StakeTable.validatorPaid`).

The rewards of an epoch are paid at its end, a validator that led `n` of its blocks
being paid `n` block rewards at once.

The node reports an error where the commission is above `maxCommission` or a product
in the computation overflows 256 bits (`StakeTable.PayoutDefined`).
-/

/--
What delegator `d` is paid of reward `r` to validator `a`: its share of `a`'s stake
times what the commission leaves of `r`, rounded down.
-/
def delegatorPaid (h : History) (a : Address) (r : Nat) (d : Address) : Nat :=
  (delegated h d a).toNat * (maxCommission - (member h a).commission) * r
    / ((member h a).stake * maxCommission)

/-- What validator `a` keeps of reward `r`: what its delegators are not paid. -/
def validatorPaid (h : History) (a : Address) (r : Nat) : Nat :=
  r - ((delegators h a).map (delegatorPaid h a r)).sum

/-- What account `x` is credited when validator `a` is paid reward `r`. -/
def credited (h : History) (a : Address) (r : Nat) (x : Address) : Nat :=
  delegatorPaid h a r x + if x = a then validatorPaid h a r else 0

/-- The node pays validator `a` reward `r` without error. -/
def PayoutDefined (h : History) (a : Address) (r : Nat) : Prop :=
  (member h a).commission ≤ maxCommission
    ∧ (maxCommission - (member h a).commission) * r < U256
    ∧ ∀ d ∈ delegators h a,
        (delegated h d a).toNat * ((maxCommission - (member h a).commission) * r) < U256

/-!
## What the specification claims

Each claim is a proposition here, proved in `StakeTableSpec.Proofs.Properties`.
The claims are grouped by the section they are about; the module documentation sums
them up as well formed, determined, fair and bounded.
-/

/-! ### The stake table -/

/-- No delegation is ever negative. -/
def DelegationsNonNegative : Prop :=
  ∀ h d a, Valid h → 0 ≤ delegated h d a

/-- A validator's stake is the sum of its delegators' delegations. -/
def StakeIsDelegations : Prop :=
  ∀ h a, stake h a = ((delegators h a).map fun d => delegated h d a).sum

/-- Stake is never negative, and fits in 256 bits. -/
def StakeBounded : Prop :=
  ∀ h a, Valid h → 0 ≤ stake h a ∧ stake h a < U256

/-- Members have distinct accounts and keys. -/
def TableDistinct : Prop :=
  ∀ h, Valid h → Distinct (table h)

/-- Nothing about a validator that exited is taken again. -/
def ExitIsFinal : Prop :=
  ∀ h a c, Valid h → Exited h a → c.validator = a → ¬ Accepts h c

/--
Each key is admitted once.

This covers the keys of validators that exited or changed their keys, which
`TableDistinct` does not: no key any validator ever held is held again.
-/
def KeysAdmittedOnce : Prop :=
  ∀ h, Valid h → (usedBls h).Nodup ∧ (usedSchnorr h).Nodup ∧ (usedX25519 h).Nodup

/-! ### From events to changes -/

/-- Events make at most one history, and it is valid. -/
def ReadsUnique : Prop :=
  ∀ es h h', Reads es h → Reads es h' → h = h' ∧ Valid h

/-! ### Reading events from L1 -/

/-- Logs with distinct places on L1 have exactly one L1 order. -/
def L1OrderFunctional : Prop :=
  ∀ (α : Type) (logs : List (LogKey × α)), (logs.map (·.1)).Nodup → Unique (InL1Order logs)

/--
A read that resumes from kept events gives the L1 order of all the logs, when the
kept events are the logs up to some block in order, and the fetched ones the logs
after it in order.
-/
def ResumeIsL1Order : Prop :=
  ∀ (α : Type) [DecidableEq α] (logs kept fetched : List (LogKey × α)) (B : Nat),
    InL1Order (logs.filter fun l => decide (l.1.1 ≤ B)) kept →
    InL1Order (logs.filter fun l => decide (B < l.1.1)) fetched →
    InL1Order logs (resume kept fetched)

/--
A read in chunks loses no log and repeats none: the logs of the chunks, one chunk
after another, are the logs of the whole range.
-/
def ChunkedReadsAll : Prop :=
  ∀ (α : Type) (logs : List (LogKey × α)) (first last size : Nat) (chunks : List (Nat × Nat)),
    Chunked first last size chunks →
    (chunks.flatMap fun c => logs.filter fun l => decide (c.1 ≤ l.1.1 ∧ l.1.1 ≤ c.2)).Perm
      (logs.filter fun l => decide (first ≤ l.1.1 ∧ l.1.1 ≤ last))

/-! ### The committee -/

/-- A stake table with distinct accounts has exactly one committee, or exactly no committee. -/
def ActiveSetFunctional : Prop :=
  ∀ usable (vs : List Member), (vs.map (·.account)).Nodup → Unique (IsActiveSet usable vs)

/--
A committee is never empty, and has `maxValidators` members, or every eligible
validator if there are fewer.
-/
def ActiveSetFull : Prop :=
  ∀ usable vs as, IsActiveSet usable vs (some as) →
    as ≠ [] ∧ as.length = min maxValidators (eligible (candidates usable vs)).length

/-- No eligible validator left out of a committee has more stake than a member. -/
def ActiveSetByStake : Prop :=
  ∀ usable vs as, IsActiveSet usable vs (some as) →
    ∀ v ∈ as, ∀ w ∈ eligible (candidates usable vs), w ∉ as → w.stake ≤ v.stake

/--
The stakes of a committee are within a factor of `vidTargetTotalStake` of each other:
each member has at least any other member's stake divided by it.
-/
def ActiveSetStakeRatio : Prop :=
  ∀ usable vs as, IsActiveSet usable vs (some as) →
    ∀ v ∈ as, ∀ w ∈ as, w.stake / vidTargetTotalStake ≤ v.stake

/--
A committee holds no more stake than the stake table it is selected from.

The node adds up a committee's stakes in 256 bits. The stake in the table is
delegated tokens, so it is within the token supply, which fits in 256 bits; then so
does the committee's total. The supply is not modelled here.
-/
def CommitteeStakeWithinTable : Prop :=
  ∀ usable h as, IsCommittee usable h as →
    (as.map (·.stake)).sum ≤ ((table h).map (·.stake)).sum

/-- The committee of a valid stake table has distinct accounts and keys. -/
def ActiveSetDistinct : Prop :=
  ∀ usable h as, IsCommittee usable h as → Distinct as

/-- The logs on L1 up to a block fix the committee read from them. -/
def CommitteeUnique : Prop :=
  ∀ usable (logs : List (LogKey × Event)) B r r',
    CommitteeAt usable logs B r → CommitteeAt usable logs B r' → r = r'

/--
Valid histories with the same commitment select the same committee, listed in the
same order, so a committee selected from a table checked against a commitment is the
network's. The tables themselves may differ in the order the validators registered in.
-/
def CommitmentFixesCommittee : Prop :=
  ∀ usable h h' r r', Valid h → Valid h' → SameCommitment h h' →
    IsActiveSet usable (table h) r → IsActiveSet usable (table h') r' → r = r'

/-! ### The leader schedule -/

/-- Entries with distinct keys have exactly one schedule order. -/
def ScheduleOrderFunctional : Prop :=
  ∀ drb (es : List Entry), (es.map (·.key)).Nodup → Unique (InScheduleOrder drb es)

/-- Every view of a committee has exactly one leader. -/
def LeaderFunctional : Prop :=
  ∀ usable h as (b : Beacon) v, IsCommittee usable h as → Unique (Leads b as v)

/--
A member of a committee leads exactly the breakpoints below the total stake that lie
in an interval as long as its stake: it leads in proportion to its stake.

The breakpoint is taken to be the view itself, so that every breakpoint is drawn.
-/
def LeaderShareIsStake : Prop :=
  ∀ usable h as (b : Beacon) m, IsCommittee usable h as → m ∈ as →
    ∃ lo, lo + m.stake ≤ (as.map (·.stake)).sum ∧
      ∀ v < (as.map (·.stake)).sum,
        Leads { b with hash := fun _ v _ => v } as v m ↔ lo ≤ v ∧ v < lo + m.stake

/--
Who leads a view does not depend on the order the committee is listed in. The listing
only gives each member its position, by which certificates name their signers.
-/
def LeaderIgnoresListing : Prop :=
  ∀ (b : Beacon) (as as' : List Member) v m, as.Perm as' → (Leads b as v m ↔ Leads b as' v m)

/-! ### The block reward and its payout -/

/-- Where the inputs give a block reward, they give exactly one. -/
def RewardFunctional : Prop :=
  ∀ supply stake avgMs, RewardDefined supply stake avgMs → Unique (BlockReward supply stake avgMs)

/--
The block rewards of a year are at most 2.13% of the supply, and half a unit per
block for rounding.

The yearly inflation is largest with the whole supply staked, where it is
`0.03 / sqrt 2` of the supply.
-/
def IssuanceBounded : Prop :=
  ∀ supply stake avgMs r, BlockReward supply stake avgMs r →
    10000 * (r * blocksPerYear avgMs) ≤ 213 * supply + 5000 * blocksPerYear avgMs

/-- More stake never lowers the block reward. -/
def RewardMonotone : Prop :=
  ∀ supply stake stake' avgMs r r', stake ≤ stake' →
    BlockReward supply stake avgMs r → BlockReward supply stake' avgMs r' → r ≤ r'

/-- A payout adds up to the reward. -/
def PayoutAddsUp : Prop :=
  ∀ h a r, Valid h → validatorPaid h a r + ((delegators h a).map (delegatorPaid h a r)).sum = r

/--
A validator with stake keeps at least its commission of a reward, and less than one
unit more per delegator.
-/
def CommissionKept : Prop :=
  ∀ h a r, Valid h → 0 < (member h a).stake → (member h a).commission ≤ maxCommission →
    (member h a).commission * r ≤ maxCommission * validatorPaid h a r
      ∧ maxCommission * validatorPaid h a r
          < (member h a).commission * r + maxCommission * (delegators h a).length

/--
Histories with the same commitment credit every account the same when a registered
validator is paid, so rewards computed from a table checked against a commitment are
the network's.
-/
def CommitmentFixesPayout : Prop :=
  ∀ h h' a r x, SameCommitment h h' → Registered h a → credited h a r x = credited h' a r x

end StakeTable
