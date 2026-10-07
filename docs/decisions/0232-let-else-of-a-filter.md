# 0232. A `let`-`else` of a `filter` tests the filter and binds what it kept

Status: Accepted. Builds on [0030](0030-option.md).

## Context

react.dev's `Link` leaves early without an `href`, `if (!href) { return
<a .. />; }`, an empty one too, then uses `href`. Ported, `let Some(href) =
href.filter(|href| !href.is_empty()) else { .. }`, it was:

```js
const href$1 = href != null && href.length !== 0 ? href : undefined;
if (href$1 == null) {
```

## Decision

**`Some(p)` of a `filter`'s `Option`, in `if let` or `let`-`else`, is the
filter's test and `p` of what it kept, with no `const` of the `Option`;
of text, `x != null && x.length !== 0` is `x`, which is falsy only
`null`, `undefined` or empty.**

```js
if (!href) {
  return <a href={href} className={className} {...props} />;
}
```

- **What it binds names what the filter kept**, where that's a variable
  that never changes; of one that does, it's a copy, as before.
- **An array keeps its `length` test**: `[]` is truthy, `!(items != null
  && items.length !== 0)`.
- **A generic `T`'s, which may be boxed (ADR 0051), is as before.**

## Why

- **It's the JS a person writes**: one test, of the variable, and no
  `href$1` beside `href`.
- **It's exact**: of a `string | undefined`, `!x` is `x == null ||
  x.length === 0`, and the filter's `Option` is `x` where its test holds.
- **It's tested**: a compiler test checks the JS of a `let`-`else` of
  `&str`, an `if let` of `String`, a `Vec`'s, and of a variable that
  changes after, and runs each, empty, missing and given.
