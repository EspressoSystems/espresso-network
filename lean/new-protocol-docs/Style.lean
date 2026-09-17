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


/-!
# House rules for print

The LaTeX counterpart of the stylesheet in `Main.lean`, for a default that
`Config` cannot reach: `TeXConfig` carries no preamble, and the only preamble
Verso assembles is the one its extensions contribute. `houseStyle` adds the
rules to every box defined below, so they outlive any one of those boxes.

Inline code cannot break. Verso computes break opportunities for the names in a
signature, a hyphen at each camel-case seam and a bare break after each dot, but
a name written as `{name X}` or as plain code reaches LaTeX without them, and a
long one then runs past the margin. `breakbefore` and `breakafter` ask fancyvrb
for the same two policies, and for a break after `_`, everywhere a name is set
inline.

A few lines still have no break in reach, a name being one long word in a narrow
column. `emergencystretch` gives TeX a last pass in which it may loosen the
spaces on such a line instead of letting it run into the margin, which is the
lesser of the two.
-/

/-!
## The fonts in print

Verso chooses three: Source Serif Pro for the body, Source Sans Pro for headings
and the table of contents, DejaVu Sans Mono for code. Each of the three settings
below replaces one of them, and `none` keeps Verso's.

A name is one fontspec resolves, so the font has to be installed, and a font for
code has to carry the symbols Lean is written in, `∀` and `→` and `⟨⟩` among
them, along with the `↪` fvextra marks a wrapped line of code with. Neither
failure stops the build: an unresolvable name falls back to the default font and
a missing symbol leaves a gap on the page, both reported in `main.log` and
nowhere else. Grep it for `Missing character` after changing any of these.

Headings and code carry their name into the HTML nowhere. There Verso leaves both
fonts to the reader's browser, through `--verso-code-font-family` and its
siblings.
-/

/-- The body font, or `none` for Verso's Source Serif Pro. -/
def bodyFont : Option String := none -- some "NewCM10-Regular"

/-- The font for headings and the table of contents, or `none` for Verso's Source Sans Pro. -/
def headingFont : Option String := none -- some "NewCMSans10-Regular"

/-- The font for code, or `none` for Verso's DejaVu Sans Mono. -/
def codeFont : Option String := none -- some "Gizmo"

private def fontRules : String :=
  choose "setmainfont" bodyFont ++ choose "setsansfont" headingFont ++ choose "setmonofont" codeFont
where
  choose (command : String) (font : Option String) : String :=
    font.map (fun name => "\\" ++ command ++ "{" ++ name ++ "}\n") |>.getD ""

private def houseTeXPreamble : String :=
  fontRules ++
  r#"
\RecustomVerbatimCommand{\LeanVerb}{Verb}{commandchars=\\\{\},fontsize=\small,breaklines=true,breakafter={._},breakbefore={ABCDEFGHIJKLMNOPQRSTUVWXYZ},breakbeforesymbolpre={-},breakaftersymbolpre={}}

\emergencystretch=1em
"#

/-- Adds the rules above to a box, whichever box it is. -/
private def houseStyle (d : BlockDescr) : BlockDescr :=
  { d with preamble := houseTeXPreamble :: d.preamble }

/-!
# The same distinctions in print

`manualMain` renders the document twice, so an extension that draws a box in HTML
has to say what that box is in LaTeX as well. A `toTeX` that only passes its
content through loses whatever the box was there for: a reproduction stops
looking like an object, a printed value comes out in the body font, and a
scenario runs into the prose around it with nothing to say where it starts or
ends.

What each box needs in the LaTeX preamble goes in `preamble`, which Verso
collects from the extensions a document uses, as it does `extraCss`. Verso's own
preamble loads `tcolorbox` and uses it for the frame around a docstring, so the
frames below are the same kind of object as those, and a reproduced definition
does not read as subordinate to a docstring discussing it.

The frame for a reproduction is shared with the one for a value, so it is defined
once here rather than in both extensions. Verso collects preamble items into a
set: two copies of this text are one item, but two that differ by a character
would both survive, and the second `\newtcolorbox` would be an error.
-/

private def specBoxTeX : String :=
  r#"
\definecolor{specFrame}{HTML}{C8C8C8}
\newtcolorbox{specBox}{
colback=white,
colframe=specFrame,
boxrule=0.4pt,
breakable,
enhanced,
left=2mm,right=2mm,top=1mm,bottom=1mm}
"#


block_extension SpecBox (mirrors : Name) via houseStyle where
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
  preamble := [specBoxTeX]
  toTeX :=
    some <| fun _ go _ _ content => do
      let content ← content.mapM fun b => do pure <| .seq #[← go b, .raw "\n"]
      pure <| .environment "specBox" #[] #[] content
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

block_extension ExpansionBox (tokens : Array ValueToken) via houseStyle where
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
  preamble := [specBoxTeX]
  toTeX :=
    some <| fun _ _ _ data _ => do
      let .ok (tokens : Array ValueToken) := FromJson.fromJson? data
        | do reportError "`{expansion}` lost its value"; pure .empty
      let text := String.join (tokens.map (·.1)).toList
      let code := Verso.Doc.TeX.escapeForVerbatim text
      pure <| .environment "specBox" #[] #[]
        #[.environment "LeanVerbatim" #[] #[] #[.raw code]]
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

block_extension ExampleBox (description : String) via houseStyle where
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
  preamble := [
    r#"
\definecolor{exampleFrame}{HTML}{98B2C0}
\definecolor{exampleTitle}{HTML}{555555}
\newtcolorbox{exampleBox}[1]{
colback=white,
colframe=exampleFrame,
colbacktitle=white,
coltitle=exampleTitle,
boxrule=0.4pt,
breakable,
enhanced,
attach boxed title to top left={xshift=2mm,yshift=-2mm},
boxed title style={top=-0.3mm,bottom=-0.3mm,left=-0.3mm,right=-0.3mm,boxrule=0.4pt},
fonttitle=\sffamily\itshape\small,
title={Example: #1}}
"#
  ]
  toTeX :=
    some <| fun _ go _ data content => do
      let description := (FromJson.fromJson? (α := String) data).toOption.getD "Example"
      let content ← content.mapM fun b => do pure <| .seq #[← go b, .raw "\n"]
      pure <| .environment "exampleBox" #[] #[.text description] content
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


/-!
# A table that fits the page

Verso renders a table to `tabular` with one `l` column per column. An `l` column
is as wide as its widest cell and cannot wrap, so a cell holding a sentence is
set on one line and the table runs past the margin, by two inches in the case of
the one in this reference.

`:::rows` takes the markup of `:::table`, an outer list of rows and an inner list
of cells in each, and renders it to `tblr`, whose columns divide the text width
between them and wrap inside a cell. The HTML is Verso's, `table.tabular`, so the
rules `Main.lean` adds for a table apply to this one as well, and the two agree
on the header: bold, with a rule under it. The cell rules come along too, since
Verso's table is no longer used in this document and its stylesheet is emitted
only for the extensions that are.

Verso's own table is left alone rather than adjusted. The alternative was to
redefine `tabular` in the preamble, which reaches every table LaTeX sets,
including the one memoir builds the title page from.
-/

/-- Whether a `:::rows` table's first row names its columns. -/
structure RowsConfig where
  header : Bool

section
variable [Monad m] [MonadError m] [MonadLiftT CoreM m]

meta def RowsConfig.parse : ArgParse m RowsConfig :=
  RowsConfig.mk <$> .flag `header false

meta instance : FromArgs RowsConfig m := ⟨RowsConfig.parse⟩
end

/-- The cells of a table, in reading order, cut into rows of `columns`. -/
private def toRows {α : Type} (columns : Nat) (cells : Array α) : Array (Array α) :=
  if columns = 0 then #[]
  else
    let rowCount := (cells.size + columns - 1) / columns
    (Array.range rowCount).map fun i => cells.extract (i * columns) ((i + 1) * columns)

block_extension RowsBox (columns : Nat) (header : Bool) via houseStyle where
  data := ToJson.toJson (columns, header)
  traverse := fun _ _ _ => pure none
  extraCss := [
    r#"
table.tabular td, table.tabular th { text-align: left; vertical-align: top; }
table.tabular td > p:first-child, table.tabular th > p:first-child { margin-top: 0; }
table.tabular td > p:last-child, table.tabular th > p:last-child { margin-bottom: 0; }
"#
  ]
  usePackages := ["\\usepackage{tabularray}"]
  preamble := [
    r#"
\definecolor{rowsRule}{HTML}{98B2C0}
"#
  ]
  toTeX :=
    some <| fun _ go _ data blocks => do
      let .ok ((columns, header) : Nat × Bool) := FromJson.fromJson? data
        | do reportError "`:::rows` lost its shape"; pure .empty
      let #[.ul cells] := blocks
        | do reportError "`:::rows` lost its cells"; pure .empty
      let rows ← (toRows columns cells).mapM fun row => do
        let rendered ← row.mapM fun cell => do
          pure <| Verso.Output.TeX.seq (← cell.contents.mapM go)
        pure <| Verso.Output.TeX.seq (rendered.toList.intersperse (.raw " & ") |>.toArray)
      let spec :=
        "width=\\linewidth,colspec={" ++ String.join (List.replicate columns "X[l]") ++ "}" ++
        ",hlines={0.4pt,rowsRule},rowsep=3pt" ++
        (if header then ",row{1}={font=\\bfseries}" else "")
      -- A `tblr` is an inline box, like the `tabular` it replaces, so without
      -- this it joins the paragraph before it and the rules meet the prose.
      pure <| .seq #[
        .raw "\\par\\medskip\\noindent\n",
        .environment "tblr" #[] #[.raw spec]
          (rows.toList.intersperse (.raw " \\\\\n") |>.toArray),
        .raw "\\par\\medskip\n"]
  toHtml :=
    open Verso.Output.Html in
    some <| fun _ go _ data blocks => do
      let .ok ((columns, header) : Nat × Bool) := FromJson.fromJson? data
        | do reportError "`:::rows` lost its shape"; pure .empty
      let #[.ul cells] := blocks
        | do reportError "`:::rows` lost its cells"; pure .empty
      let rows ← (toRows columns cells).mapIdxM fun i row => do
        let rendered ← row.mapM fun cell => do
          let content : Verso.Output.Html ← cell.contents.mapM go
          pure <| if header && i == 0 then {{<th>{{content}}</th>}} else {{<td>{{content}}</td>}}
        let row : Verso.Output.Html := .seq rendered
        pure <| if header && i == 0 then {{<thead><tr>{{row}}</tr></thead>}} else {{<tr>{{row}}</tr>}}
      pure {{<table class="tabular">{{Verso.Output.Html.seq rows}}</table>}}

open Lean.Doc.Syntax in
/--
A table whose cells may hold prose.
-/
@[directive]
meta def rows : DirectiveExpanderOf RowsConfig
  | cfg, stxs => do
    let #[list] := stxs
      | throwError "Expected a single list, whose items are the rows"
    let `(block|ul{$rowItems*}) := list
      | throwErrorAt list "Expected a single list, whose items are the rows"
    let rows ← rowItems.mapM fun rowItem => do
      let #[row] := (← listItem rowItem).filter (·.raw.isOfKind ``ul)
        | throwErrorAt rowItem "Expected one list of cells in this row"
      let `(block|ul{$cellItems*}) := row
        | throwErrorAt row "Expected one list of cells in this row"
      cellItems.mapM listItem
    let some columns := rows[0]?.map (·.size)
      | throwErrorAt list "Expected at least one row"
    if let some bad := rows.find? (·.size != columns) then
      throwErrorAt list s!"Expected {columns} cells in every row, got a row of {bad.size}"
    let cells ← rows.flatten.mapM (·.mapM elabBlock)
    ``(Block.other (RowsBox $(quote columns) $(quote cfg.header))
        #[Block.ul #[ $[Verso.Doc.ListItem.mk #[ $cells,* ]],* ]])
where
  listItem : Syntax → DocElabM (TSyntaxArray `block)
    | `(list_item| * $content*) => pure content
    | other => throwErrorAt other "Expected a list item"
