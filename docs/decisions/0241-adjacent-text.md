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

