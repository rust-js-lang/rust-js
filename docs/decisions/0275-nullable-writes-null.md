# 0275. A nullable field's `None` is `null`

Status: Accepted. Extends [0196](0196-typescript-declarations.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's errors page gives Next.js its props, a code or `null`:

```tsx
return {props: {content, toc, meta, errorCode: code, errorMessage: code ? errorCodes[code] : null}};
```

Next.js sends them as JSON, which has no `undefined`: it refuses a prop
that is. A `#[rust_js::nullable]` field took `null` (ADR 0196), but its
`None` was `undefined`, as every `None` is (ADR 0030).

## Decision

**A `#[rust_js::nullable]` field is TypeScript's `T | null` when it's
made: its `None` is `null`.**

- `None` is `null`, and a `Some(..)` or a constant is its value.
- An `Option` that may be `None` is `value ?? null`.
- One chosen by a `match` or an `if` whose arms are each `Some(..)` or
  `None` is its conditional with `null` for each `None`: `code ? errorCodes[code]
  : null`, as the errors page writes it, and `code || null` where it gives
  what it tests (`a ? a : b` is `a || b` wherever it's printed).
- Another such field's value, read or bound by a pattern and not set again,
  is `null` or a value already, as it is: `{ code, message }` of one
  destructured.

```rust
Props { code, message: None, title: None }
```

```js
return { code: code ?? null, message: null, title: undefined };
```

Its declaration stays `?: T | null`: a TypeScript caller may leave it out.

## Why

- **No program can tell**: rust-js reads `null` and `undefined` alike as
  `None` (ADR 0030); only JS that reads the object, JSON's say, sees which.
- **It's what the TypeScript type says**, `string | null`, and what the
  page wrote.
- **It's tested**: a compiler test makes one of `None`, of an `Option`, of
  a `Some`, and of another's field, and its JSON keeps each; mutations make
  each `undefined` or `?? null`.

## Since

- **A binding's parameter is nullable too, `#[rust_js::nullable(replacer)]`
  on its function**, naming it: its `None` is `null`, an `Option` that may
  be `None` is `value ?? null`, as a field's is. On the function, as a
  parameter's own attributes are dropped in an `extern` block and aren't in
  a crate's metadata. A name that isn't a parameter, or one that isn't an
  `Option`, is an error. `json::stringify_with(&value, None, 2)` is
  `JSON.stringify(value, null, 2)`, as react.dev's Sandpack template
  writes it. Case A: rust-js reads `null` and `undefined` alike as `None`,
  and `JSON.stringify` takes either as no replacer.

## Amendment: a copy of one

A nullable field given a copy of another's value, `error.rule_id.clone()`,
is given it as it is, as one given the field itself is: it's `null` where
it's `None` already, react.dev's runESLint's `ruleId: error.ruleId`.
