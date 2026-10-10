# 0356. A `const` nothing reads is no statement

Status: Accepted.

## Context

A match's bindings are `const`s of the places they name where the matched
value may change, as a library's always may (ADR 0100). A guard reads each
binding in place, before the arm's `const`s, so an arm that reads its
binding only in its guard declared one nothing reads:

```js
if (params && params.code === "3") {
  const code = params.code;
  return { notFound: true };
}
```

Next.js's `getStaticProps` had it, built by Cargo, `--library`; the same
Rust compiled alone named the place and had none.

## Decision

**A function's `const` that nothing in it reads, of a value whose making
does nothing, `!has_effects()`, is dropped, again until none is**: a
`const _kept = code;` gone leaves `code` unread too. One of a call, `const
_last = items.pop();`, stays.

## Consequences

- The corpus loses 22 lines in 7 files: Rust's unused `let`s, `const lock =
  { value: 1 };`, and a closure never called.
- The react.dev port's JS is unchanged.
