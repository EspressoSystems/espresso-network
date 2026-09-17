import Reference
open Verso.Genre.Manual

/--
Rules for tables.

Verso styles `table.tabular` with spacing and alignment only, so a table renders
without any lines at all and a header row is distinguished by nothing but the
browser's default bold. These rules add a line under the header, a hairline between rows, and an outer
border rounded like the other boxes. The borders stay `separate` because a
collapsed table ignores `border-radius`; with zero spacing the cells still sit
flush. The row selector avoids `tbody`, which Verso does not emit; a browser
inserts one, but the rule should not depend on that.

They go in `extraHead`, which reaches every page, because `extraCss` on a block
extension is emitted only on pages that use that extension.
-/
private def tableRules : String :=
  r#"
table.tabular {
  border-collapse: separate;
  border-spacing: 0;
  border: 1px solid #98b2c0;
  border-radius: 0.5rem;
  overflow: hidden;
}
table.tabular th, table.tabular td { padding: 0.35rem 0.9rem; }
table.tabular thead th { border-bottom: 1px solid #98b2c0; }
table.tabular tr + tr td { border-top: 1px solid #e4e7ea; }
"#

/--
The house style for links, extended to prose.

Verso styles a link inside code — `.hl.lean a` inherits the text colour and takes a
dotted underline — but leaves a link in prose to the browser's default blue. A
`{ref}` to a section therefore looked nothing like the reference's other links.
-/
private def linkRules : String :=
  r#"
main a { color: inherit; text-decoration: currentcolor underline dotted; }
main a:hover { text-decoration: currentcolor underline solid; }
"#

/--
Colours for Lean tokens.

Verso tokenises the code it splices — a bound variable is a `var` token, an
operator like `∨` is a `const` — but the theme gives every kind the same colour
and separates them by weight and slant alone. In a clause body full of
single-letter variables that leaves `v` and `∨` looking alike. Setting the two
custom properties the theme already reads is enough to tell them apart.
-/
private def tokenRules : String :=
  r#"
:root {
  --verso-code-const-color: #005f87;
  --verso-code-var-color: #7a4c00;
}
"#

def main := manualMain (%doc Reference)
  (config := {
    extraHead := #[.tag "style" #[] (.text false (tableRules ++ linkRules ++ tokenRules))]
  })
