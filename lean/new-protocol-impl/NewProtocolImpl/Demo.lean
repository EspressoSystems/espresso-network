module

public import NewProtocolImpl.Protocol
public meta import NewProtocolSpec.Interface
public meta import NewProtocolImpl.Protocol

/-!
# The machine, running

`Impl.next` and `Impl.State.gc` are ordinary compiled functions, and this file
runs them. Each evaluation's expected output is checked at build time, so these are
regression tests rather than illustrations — and they are the only check on the
*abstraction*, which no proof can supply: `Impl.State.abstract` is trusted glue,
so a machine that conformed while emitting nothing would satisfy every theorem in
this package and fail here.

**Hashing runs too.** `blockHash` is `@[irreducible]` rather than `opaque`, so no
proof can see through it while evaluation still goes through — it reads the
identity a block carries (`Proposal.identity`). So there is no stuck path left
here, and the machine can be driven against a real node by translating each of
its proposals with the commitment that node computed. What a deployment still
has to substitute is the signature checking the specification states as
propositions rather than code.

The runs below are the view change, the timer, the certificate store and
collection. Several emit nothing at all, which is the point of the
specification's two outputs: a step that only records what it was told owes the
network nothing.
-/

@[expose] public section

namespace NewProtocol
namespace Impl
namespace Demo

/-- A one-node configuration, anchored at genesis. -/
def anchorHeader : BlockHeader := ⟨⟨7⟩⟩

def anchorBlock : Block :=
  ⟨anchorHeader, ViewNumber.genesis, ⟨⟨⟨0⟩⟩, ViewNumber.genesis⟩, none, ⟨1⟩⟩

def cfg : Config := ⟨anchorBlock, ⟨⟨⟨0⟩⟩, ViewNumber.genesis⟩, 20⟩

def me : PubKey := ⟨1⟩

/-- We lead every view. -/
def leader : ViewNumber → Option PubKey := fun _ => some me

/-- A fresh node, holding the anchor and nothing else. -/
def node : State := initial cfg

/-- A certificate for view 4 arrives with permission to advance. -/
def advanced : State := (next cfg leader me node (Input.advanceView ⟨⟨⟨5⟩⟩, ⟨4⟩⟩)).1

-- Advancing the view emits nothing: what the node owes its own timer is not part
-- of the specification's outputs, so the whole of `StepSpec.advanceOwed` is the
-- state change below.
/-- info: [] -/
#guard_msgs in
#eval (next cfg leader me node (Input.advanceView ⟨⟨⟨5⟩⟩, ⟨4⟩⟩)).2

-- It is in view 5, and the certificate is filed alongside the anchor's.
/-- info: (5, 2) -/
#guard_msgs in
#eval (advanced.currentView.toNat, advanced.cert1s.size)

-- The local timer fires for the view it is in: the node answers with a timeout
-- vote, carrying no catch-up evidence, since it holds no lock and no timeout
-- certificate.
/--
info: [NewProtocol.Output.send
   (NewProtocol.Message.timeoutVote { data := (), view := { toNat := 5 }, signer := { toNat := 1 } } none)]
-/
#guard_msgs in
#eval (next cfg leader me advanced (Input.timeout ⟨5⟩)).2

-- …and raises its timeout bar over that view, as `SafetySpec.timeoutVoteSound` asks.
/-- info: 5 -/
#guard_msgs in
#eval (next cfg leader me advanced (Input.timeout ⟨5⟩)).1.timeoutView.toNat

-- A `Cert2` for a view whose block it does not hold: the node relays the
-- certificate (`StepSpec.cert2RelayOwed`). It does not decide — the view is
-- a gap until the block arrives, and nothing obliges the node to go get it.
/--
info: [NewProtocol.Output.send
   (NewProtocol.Message.cert2 { data := { blockHash := { toNat := 9 } }, view := { toNat := 5 } })]
-/
#guard_msgs in
#eval (next cfg leader me advanced (Input.certificate2 ⟨⟨⟨9⟩⟩, ⟨5⟩⟩)).2

-- Collecting abandons everything below the view it is in.
/-- info: 4 -/
#guard_msgs in
#eval (advanced.gc cfg).barredView.toNat

end Demo
end Impl
end NewProtocol
