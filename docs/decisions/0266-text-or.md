# 0266. Text kept where it isn't empty, or another, is ||

Status: Accepted. Extends [0030](0030-option.md) and
[0062](0062-combinators.md). Amended by
[0635](0635-a-filter-by-truth-then-a-fallback-is-or.md): preparation
writes the `||`, of every fallback. Amended: a string's `is_empty()` is
`text === ""` (rd732, the user's decision).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's Page titles a page by its meta's title, else its route's,
else nothing, by JS's `||`, which skips empty text:

```tsx
const title = meta.title || route?.title || '';
```

Rust has no falsy text, so the port says where text is kept, and what
follows where it isn't:

```rust
let title = (meta.title.filter(|title| !title.is_empty()))
    .or(route.map(|route| route.title.as_str()).filter(|title| !title.is_empty()))
    .unwrap_or("");
```

rust-js wrote each `filter` as a conditional, and `or` and `unwrap_or` as
`??`, a `const` of the first:

```js
const option = meta.title != null && meta.title.length !== 0 ? meta.title : undefined;
const title = option ?? (route?.title != null && (route?.title).length !== 0 ? route?.title : undefined) ?? "";
```

## Decision

**Text an option keeps where it isn't empty, `filter(|s| !s.is_empty())`,
then another option, `or`, or a default, `unwrap_or`, is JS's `||`**, its
parts in a chain:

```js
const title = meta.title || route?.title || "";
```

- **Text only**: JS's text is falsy where it's empty, and only there; an
  empty array is truthy, so an array's is a conditional as before.
- **What's kept is read where it is**, a variable or a property, as the
  conditional read it twice.

## Why

- **No program can tell**: `x || y` is `x` where `x` is text that isn't
  empty, and `y` where it's empty or `None`, as the filter then `or` is.
- **It's the JS a person writes.**
- **It's tested**: a compiler test chains a meta's title, a route's and
  `""`, beside an array kept where it isn't empty; mutations coalesce each,
  break the chain, and take the array for text.

## Since

- **Text's emptiness is its truthiness**: a string's `is_empty()` is
  `!text`, and `!text.is_empty()` `!!text`, the test JS writes, as text is
  falsy only where it's empty; an array's is still `length === 0`, as an
  empty one is truthy. In a test, `if`'s, a conditional's, and the parts of
  `&&` and `||` there, `!!text` is `text`, and `x != null && !!x` is `!!x`:
  `Some(code) if !code.is_empty()` is `code ? .. : ..`, as react.dev's
  errors page titles itself. A JSX child shown if text isn't empty is a
  `bool`'s, `{!!excerpt && <p />}`, where `""` would render.
- **Any value kept where it's truthy, or another, is `||`**, not text's
  alone: the test `text_or` reads is `!!x`, and `(!!x ? x : undefined) ??
  d` is `x || d` of every type. An array's emptiness, `length !== 0`,
  isn't that test, so it never was `||`; the check that the value is
  text, which kept arrays out, kept nothing out, and went.
- **Preparation writes it, of every fallback** (ADR 0635): a `filter` by
  the truth of what it holds is `x || undefined`, and `(x || undefined) ??
  y` is `x || y` where it's printed, so `or` and `or_else`, which read their
  `Option` once now, a chain, and a nullable result's `|| null` are `||`
  too. Tidy's fold of `(x ? x : undefined) ?? y` went: preparation writes
  `x ? x : undefined` as `x || undefined`. (Amended: tidy folded it only
  where the conditional was read in place.)

## Amendment: text's emptiness is its comparison with `""` (rd732, 2026-11-04, the user's decision)

The first rule of "Since", a string's `is_empty()` printing `!text`, made
React DOM's setProp `const canSetTextContent = tag !== 'body' && (tag !==
'textarea' || !!value);` once a `const` of a truth kept its `!!` (#578),
where upstream writes `value !== ''`. The user decided: **a string's
`is_empty()` is `text === ""`, and `!s.is_empty()` is `text !== ""`,
everywhere, in a test too**, `if (name === "")`, `code != null && code !==
""`, which says what the Rust says. An array's is still `length === 0`.

What reads only text's truth is the same as before. A `filter` that
keeps text where it isn't empty, then another or a default, is still
JS's `||`, `meta.title || route?.title || ""` (ADR 0635, 0651): the
`filter`'s predicate `t !== ""` is the truth of its argument
(`Expr::text_truth`, the one place that says `x !== ""` of text is `x`'s
truth, which `tests_its_truth` asks). So are a conditional of text by its
emptiness, `x === "" ? d : x` and `x !== "" ? x : d`, which are `x || d`
(tidy's `conditional`, and preparation's `a ? a : b`), the MIME type's
`blob.type || "application/octet-stream"`, and a `then_some`'s `(w) => w
|| undefined`. A `filter` of an iterator by it, which `Boolean` named, is
now `filter((p) => p !== "")`, as Rust says it.

Two react.dev sites differ from upstream, accepted by the user's decision:
its errors page's `Some(code) if !code.is_empty()`, upstream's `code ?
.. : ..` and `if (errorCode)`, is `code != null && code !== "" ? .. :
..`; and a text filter, upstream's `.filter(Boolean)`, is `.filter((g) =>
g !== "")` (`lowering-matches`, `jsx-children`, `lowering-iterators`).

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Text is `""` exactly
where it's empty, and falsy only there. Held by the corpus (34 snapshots
moved, each run against native Rust), the guide's basics.md, and the
mutations `text-empty-by-length` (kept), `text-truth-unfolded`
(`lower/combinators.ts`), `text-empty-conditional-unfolded`
(`lower/mir/tidy/logicals.ts`) and `text-truth-conditional-unfolded`
(`prepare.ts`).
