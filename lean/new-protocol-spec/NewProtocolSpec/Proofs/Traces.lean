module

public import NewProtocolSpec.Network

/-!
# Traces and their histories

How a trace's prefixes read: which step sits at an index, and what they say was
received and sent. Bookkeeping for the proofs; nothing here is part of the
contract.
-/

@[expose] public section

namespace NewProtocol
namespace Trace

variable (r : Trace)

theorem history_length (n : Nat) : (r.history n).length = n := by
  simp [history]

theorem history_getElem? {m n : Nat} (h : m < n) : (r.history n)[m]? = some (r m) := by
  simp [history, List.getElem?_range h]

theorem history_upTo {m n : Nat} (h : m ≤ n) : (r.history n).upTo m = r.history m := by
  simp only [History.upTo, history, ← List.map_take, List.take_range, Nat.min_eq_left h]

theorem mem_history {n : Nat} {st : Step} : st ∈ r.history n ↔ ∃ m, m < n ∧ r m = st := by
  simp [history]

theorem received_history {n : Nat} {i : Input} :
    (r.history n).Received i ↔ ∃ m, m < n ∧ (r m).input = i := by
  constructor
  · rintro ⟨st, hst, rfl⟩
    obtain ⟨m, hm, rfl⟩ := (mem_history r).mp hst
    exact ⟨m, hm, rfl⟩
  · rintro ⟨m, hm, rfl⟩
    exact ⟨r m, (mem_history r).mpr ⟨m, hm, rfl⟩, rfl⟩

theorem sent_history {n : Nat} {msg : Message} :
    (r.history n).Sent msg ↔ ∃ m, m < n ∧ Output.send msg ∈ (r m).output := by
  constructor
  · rintro ⟨st, hst, hmem⟩
    obtain ⟨m, hm, rfl⟩ := (mem_history r).mp hst
    exact ⟨m, hm, hmem⟩
  · rintro ⟨m, hm, hmem⟩
    exact ⟨r m, (mem_history r).mpr ⟨m, hm, rfl⟩, hmem⟩

/-- What the trace sends, some prefix records. -/
theorem sent_of_sentBy {msg : Message} (h : SentBy r msg) :
    ∃ n, (r.history (n + 1)).Sent msg ∧ Output.send msg ∈ (r n).output := by
  obtain ⟨n, hn⟩ := h
  exact ⟨n, (sent_history r).mpr ⟨n, Nat.lt_succ_self n, hn⟩, hn⟩

/-- A step's input, the prefix through it records. -/
theorem received_self (n : Nat) : (r.history (n + 1)).Received (r n).input :=
  (received_history r).mpr ⟨n, Nat.lt_succ_self n, rfl⟩

/-- What an earlier prefix received, a later one did too. -/
theorem received_mono {m n : Nat} (hle : m ≤ n) {i : Input} (h : (r.history m).Received i) :
    (r.history n).Received i := by
  obtain ⟨j, hj, hji⟩ := (received_history r).mp h
  exact (received_history r).mpr ⟨j, Nat.lt_of_lt_of_le hj hle, hji⟩

/-- What an earlier prefix sent, a later one did too. -/
theorem sent_mono {m n : Nat} (hle : m ≤ n) {msg : Message} (h : (r.history m).Sent msg) :
    (r.history n).Sent msg := by
  obtain ⟨j, hj, hjm⟩ := (sent_history r).mp h
  exact (sent_history r).mpr ⟨j, Nat.lt_of_lt_of_le hj hle, hjm⟩

end Trace
end NewProtocol
