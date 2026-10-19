module

public import NewProtocolDiff.Json
public import NewProtocolDiff.Trace
public import NewProtocolDiff.Header
public import NewProtocolDiff.Check
public import NewProtocolDiff.ProtocolCheck
public import NewProtocolDiff.PromptCheck
public import NewProtocolDiff.Coverage
public import NewProtocolDiff.Corpus
public import NewProtocolDiff.Tests
public import NewProtocolDiff.Sim
public import NewProtocolDiff.Scenario
public import NewProtocolDiff.Random

/-!
# Checking a recorded run against the specification

Scaffolding for deciding the rules of the specification on a trace another
implementation produced (`lake exe check`), and for running the machine.

* `NewProtocolDiff.Json` — `ToJson`/`FromJson` for the protocol types, derived
  so the wire format cannot drift from them, plus the injection that turns a
  recorder's identity strings into the numbers the model compares.
* `NewProtocolDiff.Trace` — the wire format: one JSON object per step, holding
  the input, what that implementation emitted for it, and when.
* `NewProtocolDiff.Check` — the signing rules of `NewProtocol.SafeHistory`.
* `NewProtocolDiff.ProtocolCheck` — the rest of `NewProtocol.ProtocolHistory`.
* `NewProtocolDiff.PromptCheck` — `NewProtocol.Prompt`, on the steps a trace has.
* `NewProtocolDiff.Coverage` — which conjuncts of `NewProtocol.Owed` a corpus of
  traces exercises (`check --coverage`).
* `NewProtocolDiff.Corpus` — what puts a trace or a delay outside what the
  specification covers.
* `NewProtocolDiff.Sim` — random runs of several machines over a simulated
  network, each history checked against the rules, the decides for agreement and
  the chain for growth (`lake exe sim`).
* `NewProtocolDiff.Scenario` — what one simulated node was given, written for a
  runner that replays it to the Rust node (`lake exe sim --scenarios`).
* `NewProtocolDiff.Tests` — the format round-trips and the checks catch what they
  claim to, checked at build time.

The checks are proved sound (`NewProtocolDiff.checkSafe_sound`,
`NewProtocolDiff.checkProtocol_sound`, `NewProtocolDiff.checkPrompt_sound`); reading
a trace and reporting on it are not. Nothing in the verified packages depends on
anything here.
-/
