# 0646. A chain held, then its fallback, is one `||`

Status: Accepted. Extends [0635](0635-a-filter-by-truth-then-a-fallback-is-or.md),
whose `(a || b || undefined) ?? c` was `a || b || c` where it was one
expression.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: no program can tell.

## Context

React's `typeName` falls back on `'Object'`:

```js
const type =
  (hasToStringTag && value[Symbol.toStringTag]) ||
  value.constructor.name ||
  'Object';
return type;
```

Its port holds the chain, then gives it through `js::cast`, the identity,
to its `unwrap_or_else`:

```rust
let r#type = tag.filter(|v| js::truthy(Some(*v))).or_else(|| name).filter(|v| js::truthy(Some(*v)));
unsafe { js::cast::<Option<String>>(r#type) }.unwrap_or_else(|| "Object".to_string())
```

and rust-js wrote `const type = a || b || undefined; return type ??
"Object";`: ADR 0635's `fallen_to` makes `(chain || undefined) ?? c` one
`||` only where the `??` is of the chain itself, not of a `const` that
holds it.

## Decision

**A `const` of a chain, read once, by the next statement's `?? c` of a
constant `c`, is the chain's `||` of `c`, `fallen_to`'s, and the read the
`const` itself**: `const type = a || b || "Object"; return type;`, as
React writes it. Preparation's `fallen_held` writes it, after a block's
expressions are, of a `return`, a `const` or a statement of the read.

- **Only a constant `c`**, which nothing can change, nor its being made
  earlier.
- **Only the one read**: a `const` read again after keeps its `undefined`
  there, `tag != null`, where the fallback would answer otherwise.

## Why

- **It's React's `typeName`**: one `||` chain, named where it's made.
- **It's the same value**: `(a || b || undefined) ?? c` is `a || b || c`,
  made once, read once.

## Consequences

- None of the corpus's or the snapshots' JS changes; the options test's
  `typeName` reads as React's.

## What holds it

- `test/lowering-options.test.ts`, "a filter by truth then a fallback is
  JS's `||`": `typeName`, and one whose `const` is read again, each run of
  every falsy value.
- Mutations in `scripts/mutations/prepare.ts`: the chain kept, and one read
  again given the fallback.
