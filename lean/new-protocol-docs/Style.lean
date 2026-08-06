import VersoManual

/-!
# A box for reproduced definitions

The definitions this reference reproduces are the specification itself, not
examples, so they should not look like the ordinary code that surrounds them.
`:::spec` draws a box around one and tints it, and carries the stylesheet that
does so — Verso collects `extraCss` from the extensions a document actually uses.

A box names the declaration it reproduces, `:::spec NewProtocol.SafeToExtend`, and
that does two jobs beyond documenting the correspondence. The name is resolved
against the environment, so a box outlives a rename no better than a
`{docstring}` splice does. And the box registers itself as that declaration's
target, which is what `{docstring}` does for the declarations it renders in full:
without it a mention of a reproduced definition would offer a reader the
docstring on hover and nowhere to go, since the only anchors on a page are the
ones some extension asked for.

Both live at the top level rather than in a namespace. Opening a namespace that
holds them is enough to exhaust the elaborator's heartbeats inside an unrelated
`{docstring}` splice, and a directive has to be in scope where it is used.

Colours are literals. Verso defines custom properties for fonts and box metrics,
which are used below, but none for code backgrounds or borders, and the generated
stylesheet carries no dark theme at all — so a `var()` here would only ever fall
back. If Verso gains those properties, these are the three places to revisit.
-/

open Verso Doc Elab Genre.Manual Output.Html
open Verso.ArgParse
open Lean


block_extension SpecBox (mirrors : Name) where
  data := ToJson.toJson mirrors.toString
  traverse := fun id data _ => do
    let .ok (name : String) := FromJson.fromJson? data
      | do reportError "`:::spec` did not record which declaration it reproduces"; pure none
    let _ ← externalTag id (← read).path name
    modify (·.saveDomainObject docstringDomain name id)
    pure none
  extraCss := [
    r#"
.spec-def {
  border: 1px solid #c8c8c8;
  border-radius: 0.5rem;
  margin: 1rem 0;
  overflow-x: auto;
}
.spec-def pre, .spec-def code {
  background: none;
  border: none;
}
"#
  ]
  toTeX :=
    some <| fun _ go _ _ content => do
      pure <| .seq <| ← content.mapM fun b => do
        pure <| .seq #[← go b, .raw "\n"]
  toHtml :=
    open Verso.Doc.Html HtmlT in
    some <| fun _ go id _ content => do
      let st : TraverseState ← HtmlT.state (genre := Verso.Genre.Manual)
      let anchor := match st.externalTags[id]? with
        | some dest => #[("id", toString dest.htmlId)]
        | none => #[]
      pure <| .tag "div" (#[("class", "spec-def")] ++ anchor) (.seq (← content.mapM go))

/-- The declaration a `:::spec` box reproduces. -/
structure SpecConfig where
  mirrors : Name

section
variable {m : Type → Type} [Monad m] [Lean.Elab.MonadInfoTree m] [MonadResolveName m]
  [MonadEnv m] [MonadError m] [MonadLiftT CoreM m]

meta def SpecConfig.parse : ArgParse m SpecConfig :=
  SpecConfig.mk <$> .positional `mirrors .resolvedName

meta instance : FromArgs SpecConfig m := ⟨SpecConfig.parse⟩
end

/--
Marks the blocks inside as a reproduction of the named definition from the
specification, rather than as example code.
-/
@[directive]
meta def spec : DirectiveExpanderOf SpecConfig
  | cfg, stxs => do
    let args ← stxs.mapM elabBlock
    ``(Block.other (SpecBox $(quote cfg.mirrors)) #[ $[ $args ],* ])


/-!
# The value of an abbreviation

`{docstring}` renders a declaration's type, which for an abbreviation is the least
interesting thing about it: `Cert1 : Type` says nothing, and `Certificate
Vote1Data` is the whole content. `{expansion NewProtocol.Cert1}` prints the value
instead, read out of the environment while this document is elaborated — so it
cannot drift from the definition, and there is nothing to keep in step.

It is for abbreviations, and only for them. A proposition whose body *is* the
specification is reproduced with `:::spec` instead: that shows it in source form,
highlighted, and pins it with an `rfl` example, which is worth the duplication for
something an audit has to read. The right-hand side of an abbreviation is a synonym
with nothing to audit, so generating it is enough.

It claims no anchor. Every declaration used with it is rendered by `{docstring}`
just above, which registers one already, and a second target for one name would
make every mention of it ambiguous. That is also why a proposition cannot use
both: `:::spec` is the anchor for the ones it reproduces.

It takes the same frame as a reproduction, unfilled. The two belong together — a
value and a definition are the same kind of thing — but an expansion continues the
entry above it rather than standing on its own, and the tint is what would make it
read as a separate object.

The value is not syntax-highlighted — highlighting works from syntax carrying
elaboration info, and this is pretty-printed from an `Expr`. Its identifiers do
link, though, which is the part that matters: the value is stored as tokens rather
than as one string, each token that names one of the value's own constants keeps
that constant's full name, and `TraverseState.linksFromDomain` turns it into a
link wherever the document has an anchor for it. A token pointing at something
outside this document — `Option`, `Unit` — simply stays text.
-/

/-- A run of the printed value: the text to show, and the constant it names. -/
private abbrev ValueToken := String × Option String

block_extension ExpansionBox (tokens : Array ValueToken) where
  data := ToJson.toJson tokens
  traverse := fun _ _ _ => pure none
  extraCss := [
    r#"
pre.spec-value {
  border: 1px solid #c8c8c8;
  border-radius: 0.5rem;
  background: none;
  padding: 0.4rem 0.8rem;
  margin: 0.5rem 0 1rem 0;
  overflow-x: auto;
}
"#
  ]
  toTeX :=
    some <| fun _ _ _ data _ => do
      let .ok (tokens : Array ValueToken) := FromJson.fromJson? data
        | do reportError "`{expansion}` lost its value"; pure .empty
      pure <| .raw <| String.join (tokens.map (·.1)).toList
  toHtml :=
    open Verso.Doc.Html HtmlT in
    some <| fun _ _ _ data _ => do
      let .ok (tokens : Array ValueToken) := FromJson.fromJson? data
        | do reportError "`{expansion}` lost its value"; pure .empty
      let st : TraverseState ← HtmlT.state (genre := Verso.Genre.Manual)
      let rendered := tokens.map fun (run, const?) =>
        let link? := const?.bind fun c =>
          (st.linksFromDomain docstringDomain c "doc" s!"Documentation for {c}")[0]?
        match link? with
        | some link => .tag "a" #[("href", link.href)] (.text true run)
        | none => .text true run
      pure <| {{<pre class="spec-value">{{.seq rendered}}</pre>}}

/-- The declaration whose value `{expansion}` prints. -/
structure ExpansionConfig where
  «of» : Name

section
variable {m : Type → Type} [Monad m] [Lean.Elab.MonadInfoTree m] [MonadResolveName m]
  [MonadEnv m] [MonadError m] [MonadLiftT CoreM m]

meta def ExpansionConfig.parse : ArgParse m ExpansionConfig :=
  ExpansionConfig.mk <$> .positional `of .resolvedName

meta instance : FromArgs ExpansionConfig m := ⟨ExpansionConfig.parse⟩
end

/-- Whether a character can occur in a printed name. -/
private def nameChar (c : Char) : Bool :=
  c.isAlphanum || c == '_' || c == '\'' || c == '.'

/--
Split printed text into name-shaped runs and the punctuation between them.

Each run is paired with the constant it names, if one of `constants` ends in it.
Matching by suffix is what lets the text show a name as the specification writes
it while the link uses the full one.
-/
private def tokenise (constants : Array Name) (text : String) : Array ValueToken :=
  let runs := text.foldl (init := (#[], "", false)) (fun (out, cur, wasName) c =>
    let isName := nameChar c
    if isName == wasName then (out, cur.push c, isName)
    else (if cur.isEmpty then out else out.push (cur, wasName), c.toString, isName))
  let runs := (if runs.2.1.isEmpty then runs.1 else runs.1.push (runs.2.1, runs.2.2))
  runs.map fun (run, isName) =>
    if !isName then (run, none)
    else
      let hit := constants.find? fun n =>
        n.toString == run || n.toString.endsWith s!".{run}"
      (run, hit.map (·.toString))

/-- What the named abbreviation stands for, taken from the environment. -/
@[block_command]
meta def expansion : BlockCommandOf ExpansionConfig
  | cfg => do
    let info ← getConstInfo cfg.of
    let some value := info.value?
      | throwError m!"'{cfg.of}' has no value, so there is nothing to expand"
    -- Binders move to the left of the `:=`, as the source writes them, rather
    -- than staying as the `fun` the value really is. Their types are not
    -- repeated: the signature is rendered by `{docstring}` directly above.
    let shown ← Lean.Meta.lambdaTelescope value fun binders body => do
      let names ← binders.mapM fun b => do pure (← b.fvarId!.getUserName).toString
      let lhs := String.intercalate " " (cfg.of.getString! :: names.toList)
      pure s!"{lhs} := {(← Lean.Meta.ppExpr body).pretty}"
    -- Shown as the specification's own modules write it. They are all inside
    -- `namespace NewProtocol`, whereas here `Block` and `Output` are names in
    -- Verso's genre too, so the pretty-printer qualifies those two and nothing
    -- else. The link keeps the full name regardless; only the text is shortened.
    let tokens := tokenise value.getUsedConstants (shown.replace "NewProtocol." "")
    ``(Block.other (ExpansionBox $(quote tokens)) #[])


/-!
# A collapsible box for scenarios

A rule that forbids something is easiest to judge against a run in which the
thing would otherwise happen. Such a walk-through is longer than the rule and
not part of it, so `:::scenario "title"` folds it away: the reader sees the
title and opens it if the rule is not already obvious.

The markup and the `Example: ` prefix follow Lean's own reference manual, so a
reader who has met one there recognises it here.
-/

block_extension ExampleBox (description : String) where
  data := ToJson.toJson description
  traverse := fun _ _ _ => pure none
  extraCss := [
    r#"
details.example {
  border: 1px solid #98b2c0;
  border-radius: 0.5rem;
  margin: var(--verso--box-vertical-margin, 1rem) 0;
  clear: both;
}
/* Folded, the box is just its own title, so it reads as a control rather than
   as prose: filled, and lifting on hover. */
details.example:not([open]) {
  background-color: #f2f3f5;
}
details.example:not([open]):hover {
  background-color: #eaecef;
}
details.example > summary.description {
  font-style: italic;
  font-family: var(--verso-structure-font-family);
  padding: var(--verso--box-padding, 0.5rem 0.8rem);
  cursor: pointer;
}
details.example > summary.description::before { content: "Example: "; }
details.example[open] > summary.description { margin-bottom: 0; }
details.example > .example-content { padding: 0 0.8rem 0.8rem; }
details.example > .example-content > :first-child { margin-top: 0; }
details.example > .example-content > p:last-child { margin-bottom: 0; }
"#
  ]
  toTeX :=
    some <| fun _ go _ _ content => do
      pure <| .seq <| ← content.mapM fun b => do
        pure <| .seq #[← go b, .raw "\n"]
  toHtml :=
    open Verso.Output.Html in
    some <| fun _ go _ data content => do
      let description := (FromJson.fromJson? (α := String) data).toOption.getD "Example"
      pure <| {{
        <details class="example">
          <summary class="description">{{description}}</summary>
          <div class="example-content">{{← content.mapM go}}</div>
        </details>
      }}

/-- The title of a `:::scenario`, which is all a folded one shows. -/
structure ScenarioConfig where
  description : String

section
variable [Monad m] [MonadError m]

meta def ScenarioConfig.parse : ArgParse m ScenarioConfig :=
  ScenarioConfig.mk <$> .positional `description .string

meta instance : FromArgs ScenarioConfig m := ⟨ScenarioConfig.parse⟩
end

/--
A run that shows why a rule is there, folded away until the reader wants it.
-/
@[directive]
meta def scenario : DirectiveExpanderOf ScenarioConfig
  | cfg, stxs => do
    let args ← stxs.mapM elabBlock
    ``(Block.other (ExampleBox $(quote cfg.description)) #[ $[ $args ],* ])
