# 0266. Text kept where it isn't empty, or another, is ||

Status: Accepted. Extends [0030](0030-option.md) and
[0062](0062-combinators.md).

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
