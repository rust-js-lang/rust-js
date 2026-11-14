# 0241. Text right before text is in braces

Status: Accepted. Extends [0040](0040-jsx.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's Challenge writes two texts side by side:

```jsx
Challenge {currentChallenge.order} of{' '}
{totalChallenges}
```

Its children are `" of"` and `" "`, two text nodes, which React renders
apart: `1<!-- --> of<!-- --> <!-- -->4`. rust-js wrote each string child
as JSX text, so the two were `of ` together: one text, one node, a page
that differs.

## Decision

**A string child is JSX text, but not right before a child that is JSX
text: then it's in braces.**

```rust
jsx! { <p>{"Challenge"}{" "}{order}{" of"}{" "}{total}</p> }
```

```jsx
<p>
  {"Challenge"} {order}
  {" of"} {total}
</p>
```

- **The earlier is in braces, not the later**: oxfmt, as Prettier, makes
  a `{" "}` within a line a space, of the text before it, which would join
  them again. A text after a string in braces stays text.
- **It's read from the last child back**, so of three, `a{"b"}c`.

## Why

- **It renders what React renders**: each string, a text node of its own.
- **It's the JSX a person writes**: text as text, braces only where JSX
  needs them.
- **It's tested**: a JSX test renders adjacent strings to HTML, with
  React's marks between text nodes; a mutation writes every string as
  text.

## Amendment: the later in braces, where a person writes it so

Of two strings side by side, the later is in braces where it has a space
at an edge and isn't blank, and the earlier has none: react.dev's 404
page writes `If this is a mistake{', '}`, which was
`{"If this is a mistake"}, `. A blank one, `{" "}`, stays text, the
earlier in braces, as a formatter makes a later `{" "}` a space of the
text before it. A JSX test renders both; mutations always brace the
earlier, and brace a blank later.

## Amendment (2026-11-14): a string is JSX text only where its whitespace survives (#1807, P0; Codex's rule)

A string child was JSX text wherever JSX could write its characters, and
JSX text doesn't carry all whitespace: JSX makes a tab a space, a
formatter lays a run of spaces out as one, and one breaking a line at a
text's edge space writes it `{" "}`, a text node of its own. So
`jsx! { <pre>{"  "}<b>{"let"}</b>{"\tend\t"}</pre> }` rendered a space
short with its tab a space, a code editor a column off, and
`{" 0.55 + 0.45 * "}` between two spans became `[span, " 0.55 + 0.45 *",
" ", span]`, the same markup, two text nodes, a text advance off in the
browser. **A string child is JSX text only where it's a single space, or
has no space at its start or end, no run of spaces and no whitespace but
spaces** (`to_oxc::expressions::jsx_text`); any other is in braces, its
whitespace whole. An attribute's string keeps its whitespace as JSX
writes it, so attributes ask what JSX can write alone (`jsx_text_safe`).

The amendment below, "the later in braces", is gone: a string of a space
at its edge is never JSX text now, so its loop never chose. Text right
before text is still in braces, as decided above.

What it costs: text beside an interpolation is braced,
`<p>{"Clicked "}{count}{" times"}</p>`, where the same text on one line,
`Clicked {count} times`, gives React the same children. The printer, which
decides where a line breaks, is to put in braces only the child a break
touches, so the rest is JSX text again (#1807's follow-up).

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, a page
laid out otherwise, so Rust's strings are kept whole. Held by
`test/jsx.test.ts`'s "a string child keeps its whitespace", which compares
React's children arrays and markup with React given the same strings
natively, Codex's two cases among them, and the mutations
`jsx-text-space-runs`, `jsx-text-edge-spaces` and `jsx-text-tabs`.

## Amendment (2026-11-14): the printer never breaks at a text's edge space (#1807's follow-up)

The strict rule above braced every string of a space at its edge, so the
plainest JSX was braces, `<p>{"Clicked "}{count}{" times"}</p>`, where the
same text on one line gives React the same children: only a line break at
the edge space split a text node. The printer owns where lines break, so
it never breaks there now: rust-js's oxc formatter (`vendor/oxc_formatter`,
its one change in `print/jsx/child_list.rs`) writes the whitespace between
a text's word and an element or an expression beside it as a plain space,
where upstream wrote `{" "}` and a line break wherever the children didn't
fit. A break between two words of a text stays, as JSX joins the lines
with a space. So a string with a space at its edge beside another child is
JSX text again, `<p>Clicked {count} times</p>`, and the amendment above's
loop, the later in braces, `If this is a mistake{", "}`, chooses again. At
the start or end of an element's children a line break beside the tag
would drop it, so there it's still in braces, `{" lead"}`, and under the
app's Prettier, which breaks there, every one is (ADR 0381, amended).

What it costs: a text glued to the element beside it can make a line
longer than the print width, five lines of the guide's JSX and the
snapshots', none of the corpus'.

**It's checked of every layout.** `format::jsx_shape` reads each JSX
element's children as React is given them, JSX text by JSX's own rules,
Babel's `cleanJSXElementLiteralChild`; rust-js asserts of each JSX module
it lays out that its children are what they were, an internal error
otherwise, so the corpus, the guide and the port prove it on every build,
and a hook's layout that changes them is refused (ADR 0381, amended). The
assertion caught a bug of its own reading while it was written. Held by
`test/jsx.test.ts`'s "a string child keeps its whitespace" and
`test/jsx-text-browser.test.ts`, which counts each case's text nodes and
measures its width in Chromium against React given the same strings,
Codex's four cases, a space at a text's start, its end, both, and a formula
that wraps; and by the mutations of `format/jsx_shape.ts`.


## Amendment (2026-11-15): a blank later is braced as any spaced one is

The first amendment kept a blank later string, `{" "}`, out of braces,
as a formatter joins a lone `{" "}` to the text before it. Since #1807 a
lone `{" "}` beside text braces that text again (the lone-space rule),
so that guard decided nothing for `" "`, and main's mutation run showed
it (`adjacent-text-blank-braced` survived). It's gone: a blank later of
any other kind, `{"  "}`, which no formatter joins, is braced as any
spaced string is, its earlier text JSX text, `Challenge{"  "}`, the same
children React is given. Held by jsx-children's "adjacent text children
stay apart", in React's render.
