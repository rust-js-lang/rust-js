# oxc_formatter, with #1807's change

oxc's `crates/oxc_formatter` at the tag rust-js's oxc crates are pinned to,
`crates_v0.151.0` (bec650f15b457f3bffb6d79b65bf74bfcbaecdce), MIT-licensed
(`LICENSE`), its `src` alone, with one change, in
`src/print/jsx/child_list.rs`: the whitespace between a JSX text's word and
an element or an expression beside it is a plain space, never where a line
breaks. Upstream, it's `JsxSpace`, a space where the children fit on a line
and `{" "}` and a line break where they don't: one text `"Clicked "`, its
space at its edge, became two text nodes, `"Clicked"` and `" "`, the same
markup and another layout, a text advance off in a browser (#1807, ADR 0241,
amended). A break between two words of a text stays, as JSX joins the lines
with a space. What a text has at the start or end of an element's children
rust-js writes in braces, as a line break beside the tag would drop it.

Cargo's `[patch]` in the root `Cargo.toml` puts it in place of oxc's own.
Its manifest is written for a crate outside oxc's workspace: the same
dependencies, at the same tag, but its build script, which made oxc's own
tests. `test/architecture.test.ts` holds its tag to the root's, as
`oxc_formatter_core`'s.

The change is held where its answer is seen: rust-js asserts of every JSX
module it lays out that each element's children are what they were, as
React is given them (`src/format/jsx_shape.rs`), and `test/jsx.test.ts`'s
"a string child keeps its whitespace" and `test/jsx-text-browser.test.ts`
lay out texts that would break there. Mutations reach only the compiler's
own crate, so none is of this file.

