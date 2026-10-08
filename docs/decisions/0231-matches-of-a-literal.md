# 0231. A `matches!` of a kind's literal is the literal's test, in place

Status: Accepted. Builds on [0193](0193-some-of-a-unit-variant.md) and
[0214](0214-untagged-enums.md).

Case: N, C ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's `Link` tests a child's type, `child.type?.mdxName ===
'inlineCode'`. Ported, `matches!(js::get(element.r#type(), "mdxName")
.map(js::classify), Some(js::Kind::String("inlineCode")))`, it was:

```js
const unknown$1 = child.type.mdxName;
const match = unknown$1 != null ? unknown$1 : undefined;
if (match != null && typeof match === "string" && match === "inlineCode") {
```

## Decision

**An untagged variant's test of a literal of its kind is the literal's
test; a `matches!` with no binding nor guard reads what it tests where it
tests it, once and first; and `Option::map` of what gives back its
argument is the value, where it's tested.**

```js
if (child.type.mdxName === "inlineCode") {
```

- **`x === "a"` says `typeof x === "string"`**, and a number's or a
  `bool`'s literal its kind: `Kind::String("a")` is `x === "a"`, and
  then `Some(..)` of it needs no `!= null` (ADR 0193).
- **`!= null` before a `typeof` of a kind no `null` is goes**:
  `Some(Kind::String(_))` is `typeof x === "string"`, as an or-pattern's
  is.
- **What lowering the subject put in `const`s goes in the test**, last
  first, while the test reads it once, as what it reads first: JS reads
  it where the `const` did, before anything else of the test.
- **`(u != null ? u : undefined) === "a"` is `u === "a"`**, and its
  `typeof` `u`'s: `.map(js::classify)` turns `null` into `undefined`,
  and neither is `"a"` nor a `"string"`.
- **A temporary with a destructor stays in its `const`**, which its drop
  names: `matches!(make(n).0, 3)` where `make` gives a `Drop` type.

## Why

- **It's the JS a person writes**, and it's exact: a strict equality
  with a string is false of every other kind and of `null`.
- **The order is Rust's**: what the test reads first is what the `const`
  read, and in a `format!`, `matches!(deep, ..)` reads `deep` after the
  arguments before it, as Rust does, where the `const` read it before.
- **It's tested**: a compiler test checks the JS of `Kind::String("a")`
  of a property, of an `Option`, and of a value, and of
  `Some(Kind::String(_))`, runs each on strings, numbers, `null` and
  missing properties, and runs a `matches!` of a temporary with a
  destructor.
