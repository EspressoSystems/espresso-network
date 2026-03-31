● Here are all the usages of the HotShot Consensus struct outside crates/hotshot/:

---

sequencer/src/catchup.rs — 3 writes to HotShot Consensus

Imports hotshot_types::consensus::Consensus directly. Three functions take &Arc<RwLock<Consensus<SeqTypes>>> and write
to it:

┌───────────┬─────┬───────────────────────────────────┬──────────────────────────────────────────────────────────┐ │
Line │ R/W │ Function │ What it does │
├───────────┼─────┼───────────────────────────────────┼──────────────────────────────────────────────────────────┤ │
1643-1650 │ W │ add_fee_accounts_to_state() │ Writes fee account proofs into validated_state_map │
├───────────┼─────┼───────────────────────────────────┼──────────────────────────────────────────────────────────┤ │
1700-1707 │ W │ add_v2_reward_accounts_to_state() │ Writes v2 reward account proofs into validated_state_map │
├───────────┼─────┼───────────────────────────────────┼──────────────────────────────────────────────────────────┤ │
1757-1764 │ W │ add_v1_reward_accounts_to_state() │ Writes v1 reward account proofs into validated_state_map │
└───────────┴─────┴───────────────────────────────────┴──────────────────────────────────────────────────────────┘

These are the only writes to HotShot Consensus from outside crates/hotshot/.

---

sequencer/src/api.rs — ~20 reads via double-indirection

The sequencer's Consensus<N, P> is SystemContextHandle, so access goes: handle.read().await → SystemContextHandle →
.consensus() → Arc<RwLock<hotshot Consensus>> → .read().await

┌────────────────────────┬─────┬────────────────────────────────────────────────────────────────────────────┐ │ Lines │
R/W │ What's read │
├────────────────────────┼─────┼────────────────────────────────────────────────────────────────────────────┤ │ 436-443
│ R │ current_proposal_participation() │
├────────────────────────┼─────┼────────────────────────────────────────────────────────────────────────────┤ │ 448-455
│ R │ previous_proposal_participation() │
├────────────────────────┼─────┼────────────────────────────────────────────────────────────────────────────┤ │ 460-467
│ R │ current_vote_participation() │
├────────────────────────┼─────┼────────────────────────────────────────────────────────────────────────────┤ │ 472-479
│ R │ previous_vote_participation() │
├────────────────────────┼─────┼────────────────────────────────────────────────────────────────────────────┤ │ 869-876
│ R │ Gets consensus() Arc, then calls add_fee_accounts_to_state (→ write) │
├────────────────────────┼─────┼────────────────────────────────────────────────────────────────────────────┤ │ 963-970
│ R │ Gets consensus() Arc, then calls add_v2_reward_accounts_to_state (→ write) │
├────────────────────────┼─────┼────────────────────────────────────────────────────────────────────────────┤ │
1010-1017 │ R │ Gets consensus() Arc, then calls add_v1_reward_accounts_to_state (→ write) │
├────────────────────────┼─────┼────────────────────────────────────────────────────────────────────────────┤ │
1062-1065 │ R │ decided_state() for fee merkle tree │
├────────────────────────┼─────┼────────────────────────────────────────────────────────────────────────────┤ │
1082-1085 │ R │ decided_state() for blocks frontier │
├────────────────────────┼─────┼────────────────────────────────────────────────────────────────────────────┤ │ 1100 │ R
│ decided_state() for chain config │
├────────────────────────┼─────┼────────────────────────────────────────────────────────────────────────────┤ │
1112-1119 │ R │ undecided_leaves() for leaf chain │
├────────────────────────┼─────┼────────────────────────────────────────────────────────────────────────────┤ │
1161-1164 │ R │ decided_state() for v2 reward tree │
├────────────────────────┼─────┼────────────────────────────────────────────────────────────────────────────┤ │
1183-1186 │ R │ decided_state() for v1 reward tree │
├────────────────────────┼─────┼────────────────────────────────────────────────────────────────────────────┤ │ 329-332
│ R │ cur_epoch() for highest epoch │
├────────────────────────┼─────┼────────────────────────────────────────────────────────────────────────────┤ │ 343-346
│ R │ membership_coordinator │
├────────────────────────┼─────┼────────────────────────────────────────────────────────────────────────────┤ │ 356, 386
│ R │ cur_epoch() for stake table endpoints │
├────────────────────────┼─────┼────────────────────────────────────────────────────────────────────────────┤ │ 399-402
│ R │ membership_coordinator for reward per epoch │
├────────────────────────┼─────┼────────────────────────────────────────────────────────────────────────────┤ │ 421-424
│ R │ membership_coordinator for authenticated validator map │
├────────────────────────┼─────┼────────────────────────────────────────────────────────────────────────────┤ │ 3470,
3542, 3624, 3631 │ R │ Tests: decided_state(), decided_leaf() │
└────────────────────────┴─────┴────────────────────────────────────────────────────────────────────────────┘

---

sequencer/src/state_signature.rs — 1 read (via SystemContextHandle)

┌─────────┬─────┬──────────────────────────────────────────────────────────────────────────────┐ │ Line │ R/W │ What's
read │ ├─────────┼─────┼──────────────────────────────────────────────────────────────────────────────┤ │ 110-113 │ R │
consensus_state.read().await → reads epoch_height and membership_coordinator │
└─────────┴─────┴──────────────────────────────────────────────────────────────────────────────┘

Note: this reads the SystemContextHandle, not the inner Consensus struct directly. It accesses consensus.epoch_height
and consensus.membership_coordinator which are fields on the handle.

---

sequencer/src/proposal_fetcher.rs — 1 write + 2 reads (via handle → inner Consensus)

┌─────────┬─────┬───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ Line │ R/W │ What's accessed │
├─────────┼─────┼───────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ 134 │ R │ handle.consensus.read().await.event_stream() (reads handle, not inner Consensus) │
├─────────┼─────┼───────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ 188 │ R │ handle.consensus.read().await.request_proposal() (reads handle) │
├─────────┼─────┼───────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ 200-204 │ W │ handle.consensus() → consensus.write().await — writes leaf into validated_state_map via update_leaf() │
└─────────┴─────┴───────────────────────────────────────────────────────────────────────────────────────────────────────┘

---

sequencer/src/request_response/data_source.rs — 3 writes + 1 read

┌──────┬─────┬─────────────────────────────────────────────────────────────┐ │ Line │ R/W │ What's accessed │
├──────┼─────┼─────────────────────────────────────────────────────────────┤ │ 99 │ W │ Passes consensus.consensus() to
add_fee_accounts_to_state() │ ├──────┼─────┼─────────────────────────────────────────────────────────────┤ │ 115 │ R │
consensus.consensus().read().await.undecided_leaves() │
├──────┼─────┼─────────────────────────────────────────────────────────────┤ │ 243 │ W │ Passes to
add_v2_reward_accounts_to_state() │ ├──────┼─────┼─────────────────────────────────────────────────────────────┤ │ 288 │
W │ Passes to add_v1_reward_accounts_to_state() │
└──────┴─────┴─────────────────────────────────────────────────────────────┘

---

sequencer/src/request_response/recipient_source.rs — 1 read

┌───────┬─────┬────────────────────────────────────────────────┐ │ Line │ R/W │ What's accessed │
├───────┼─────┼────────────────────────────────────────────────┤ │ 34-37 │ R │
consensus.consensus().read().await.cur_epoch() │ └───────┴─────┴────────────────────────────────────────────────┘

---

Summary outside crates/hotshot/

┌────────────────────────────────────────────────────┬───────┬─────────────────────────────┐ │ File │ Reads │ Writes │
├────────────────────────────────────────────────────┼───────┼─────────────────────────────┤ │ sequencer/src/api.rs │
~20 │ 3 (via catchup helpers) │
├────────────────────────────────────────────────────┼───────┼─────────────────────────────┤ │ sequencer/src/catchup.rs
│ 0 │ 3 (direct Consensus writes) │
├────────────────────────────────────────────────────┼───────┼─────────────────────────────┤ │
sequencer/src/proposal_fetcher.rs │ 2 │ 1 │
├────────────────────────────────────────────────────┼───────┼─────────────────────────────┤ │
sequencer/src/request_response/data_source.rs │ 1 │ 3 (via catchup helpers) │
├────────────────────────────────────────────────────┼───────┼─────────────────────────────┤ │
sequencer/src/request_response/recipient_source.rs │ 1 │ 0 │
├────────────────────────────────────────────────────┼───────┼─────────────────────────────┤ │
sequencer/src/state_signature.rs │ 1 │ 0 │
└────────────────────────────────────────────────────┴───────┴─────────────────────────────┘

The external writes are concentrated in the catchup/state-repair path (inserting fetched merkle proofs into
validated_state_map) and the proposal fetcher (inserting fetched leaves). The reads are predominantly for API serving —
participation stats, decided state, undecided leaves, and epoch/membership info.
