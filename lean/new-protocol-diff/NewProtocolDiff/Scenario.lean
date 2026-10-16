module

public import NewProtocolDiff.Sim
public import Lean.Data.Json

/-!
# Scenarios for the Rust node

What one node of a simulated run (`NewProtocolDiff.Sim`) was given, in order and
with its time, written for a runner that replays it to the Rust node.

A scenario names blocks and certificates abstractly: a block by the identity the
simulated network gave it, a certificate by the block it is over, its epoch,
height and view. The runner builds the real objects, so the format carries what
it needs to build them and nothing that only a real object has, such as a
signature. Identities are plain numbers, unlike in a trace, where they are the
recorder's text.

The first line describes the run, the second lists every block the network
built, and each further line is one input:

    {"scenario": {"node": 3, "nodes": 10, "epochHeight": 10, ...}}
    {"blocks": [{"id": 2, "view": 1, "parent": {"block": 1, ...}, ...}, ...]}
    {"t": 30, "timer": true, "input": {"timeout": 1}}

The block list is there because a certificate can reach a node before the
proposal of its block, or without it, and the runner needs the block to build the
certificate. Block `1` is the anchor, which every run starts from and the list
leaves out.
-/

@[expose] public section

namespace NewProtocolDiff
namespace Scenario

open Lean NewProtocol Sim

/-- A certificate over a block. -/
def certJson (blockHash : BlockHash) (epoch : EpochNumber) (height : BlockNumber) (view : ViewNumber) :
    Json :=
  Json.mkObj [("block", toJson blockHash.toNat), ("epoch", toJson epoch.toNat),
    ("height", toJson height.toNat), ("view", toJson view.toNat)]

def cert1Json (c : Cert1) : Json := certJson c.data.blockHash c.data.epoch c.data.blockNumber c.view

def cert2Json (c : Cert2) : Json := certJson c.data.blockHash c.data.epoch c.data.blockNumber c.view

def timeoutCertJson (tc : TimeoutCert) : Json :=
  Json.mkObj [("epoch", toJson tc.data.epoch.toNat), ("view", toJson tc.view.toNat),
    ("lock", cert1Json tc.data.lock)]

def evidenceJson : Option TimeoutCert → Json
  | some tc => timeoutCertJson tc
  | none => Json.null

def blockJson (p : Proposal) (valid : Bool := true) : Json :=
  Json.mkObj [("id", toJson p.identity.toNat), ("valid", toJson valid),
    ("view", toJson p.viewNumber.toNat),
    ("epoch", toJson p.epoch.toNat), ("height", toJson p.blockHeader.blockNumber.toNat),
    ("payload", toJson p.blockHeader.payloadCommit.toNat), ("parent", cert1Json p.parentCert),
    ("evidence", evidenceJson p.timeoutEvidence)]

def inputJson : Input → Json
  | .blockReconstructed v c =>
    Json.mkObj [("blockReconstructed",
      Json.mkObj [("view", toJson v.toNat), ("payload", toJson c.toNat)])]
  | .certificate1 c => Json.mkObj [("certificate1", cert1Json c)]
  | .certificate2 c => Json.mkObj [("certificate2", cert2Json c)]
  | .epochChange c1 c2 p =>
    Json.mkObj [("epochChange",
      Json.mkObj [("cert1", cert1Json c1), ("cert2", cert2Json c2), ("block", blockJson p)])]
  | .headerBuilt v parent h =>
    Json.mkObj [("headerBuilt",
      Json.mkObj [("view", toJson v.toNat), ("parent", toJson parent.toNat),
        ("payload", toJson h.payloadCommit.toNat), ("height", toJson h.blockNumber.toNat)])]
  | .proposal sender p share =>
    Json.mkObj [("proposal",
      Json.mkObj [("sender", toJson sender.toNat), ("block", blockJson p),
        ("share", toJson share.isSome)])]
  | .revote sender r =>
    Json.mkObj [("revote",
      Json.mkObj [("sender", toJson sender.toNat), ("cert", cert1Json r.cert),
        ("view", toJson r.view.toNat), ("evidence", evidenceJson r.timeoutEvidence)])]
  | .blockValidated v h =>
    Json.mkObj [("blockValidated", Json.mkObj [("view", toJson v.toNat), ("block", toJson h.toNat)])]
  | .timeout v => Json.mkObj [("timeout", toJson v.toNat)]
  | .timeoutCertificate tc => Json.mkObj [("timeoutCertificate", timeoutCertJson tc)]
  | .timeoutOneHonest v => Json.mkObj [("timeoutOneHonest", toJson v.toNat)]

/-- The first line: the run's parameters and the node the scenario is for. -/
def headerJson (P : Params) (k : PubKey) : Json :=
  Json.mkObj [("scenario", Json.mkObj
    [("node", toJson k.toNat), ("nodes", toJson P.nodes), ("faulty", toJson P.faulty),
     ("equivocators", toJson P.equivocators), ("misbehaving", toJson P.misbehaving),
     ("committees", toJson P.committees), ("epochHeight", toJson P.epochHeight),
     ("seed", toJson P.seed), ("gst", toJson P.gst), ("delta", toJson P.delta),
     ("tau", toJson P.tau)])]

/-- The second line: every block the network built, in the order it built them. -/
def blocksJson (s : State) : Json :=
  Json.mkObj [("blocks", Json.arr (s.blocks.reverse.map fun b =>
    blockJson b (!s.invalid.contains b.identity.toNat)).toArray)]

/-- The scenario of node `k` in a run that ended in state `s`. -/
def render (P : Params) (s : State) (k : PubKey) : String :=
  let lines := ((s.log[k.toNat - 1]?).getD []).reverse.map fun (t, timer, i) =>
    Json.mkObj [("t", toJson t), ("timer", toJson timer), ("input", inputJson i)]
  String.join ((headerJson P k :: blocksJson s :: lines).map fun j => j.compress ++ "\n")

end Scenario
end NewProtocolDiff
