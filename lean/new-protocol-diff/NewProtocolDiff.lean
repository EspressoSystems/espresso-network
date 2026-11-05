module

public import NewProtocolDiff.Json
public import NewProtocolDiff.Trace
public import NewProtocolDiff.Header
public import NewProtocolDiff.Check
public import NewProtocolDiff.ProtocolCheck
public import NewProtocolDiff.Replay
public import NewProtocolDiff.Corpus
public import NewProtocolDiff.Tests
public import NewProtocolDiff.Sim

/-!
# Differential testing against a recorded run

Scaffolding for driving `NewProtocol.Impl.next` with a trace another
implementation produced, and reporting where the two part company.

* `NewProtocolDiff.Json` — `ToJson`/`FromJson` for the protocol types, derived
  so the wire format cannot drift from them, plus the injection that turns a
  recorder's identity strings into the numbers the model compares.
* `NewProtocolDiff.Trace` — the wire format: one JSON object per step, holding
  the input and what that implementation emitted for it.
* `NewProtocolDiff.Replay` — the comparison, which is *not* output equality;
  see `Marks` for what it is and why.
* `NewProtocolDiff.Corpus` — which divergences a boundary of the specification
  excuses, and the verdict on each trace.
* `NewProtocolDiff.Sim` — random runs of several machines over a simulated
  network, each history checked against the rules, the decides for agreement and
  the chain for growth (`lake exe sim`).
* `NewProtocolDiff.Tests` — the format round-trips and the comparison catches
  what it claims to, checked at build time.

Nothing here is verified, and nothing here is depended on by the packages that
are. It is a test harness that happens to be written in Lean because the
machine it drives is.
-/
