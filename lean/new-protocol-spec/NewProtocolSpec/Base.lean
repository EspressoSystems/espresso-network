module

@[expose] public section

namespace NewProtocol

/-- The index of a view. -/
structure ViewNumber where
  /-- A `ViewNumber` wraps a `Nat`. -/
  toNat : Nat
deriving DecidableEq, Repr, Inhabited, Ord

namespace ViewNumber

def genesis : ViewNumber := ⟨0⟩

instance : HAdd ViewNumber Nat ViewNumber := ⟨fun v n => ⟨v.toNat + n⟩⟩

instance : HSub ViewNumber Nat ViewNumber := ⟨fun v n => ⟨v.toNat - n⟩⟩

instance : LE ViewNumber := ⟨fun a b => a.toNat ≤ b.toNat⟩

instance : LT ViewNumber := ⟨fun a b => a.toNat < b.toNat⟩

instance (a b : ViewNumber) : Decidable (a ≤ b) :=
  inferInstanceAs (Decidable (a.toNat ≤ b.toNat))

instance (a b : ViewNumber) : Decidable (a < b) :=
  inferInstanceAs (Decidable (a.toNat < b.toNat))

instance (n : Nat) : OfNat ViewNumber n := ⟨⟨n⟩⟩

instance : Max ViewNumber := ⟨fun a b => if a ≤ b then b else a⟩

instance : Min ViewNumber := ⟨fun a b => if a ≤ b then a else b⟩

end ViewNumber

end NewProtocol
