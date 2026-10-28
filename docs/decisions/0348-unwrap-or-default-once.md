# 0348. `unwrap_or_default()` of a call is its `??`

Status: Accepted. Amends [0326](0326-option-and-result-methods.md)'s
`unwrap_or_default`; the boxed `Some` of [0051](0051-generic-options.md)
is unchanged.

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`get(n).unwrap_or(String::new())` was `get(n) ?? ""`, but
`get(n).unwrap_or_default()` kept the call in a `const` first:

```js
const option = get(n);
return option ?? "";
```

The `const` is there for the combinators that read their `Option` twice,
`option != null ? f(option) : d`. An `Option` whose `Some` isn't boxed is
read once by `??`, as `unwrap_or`'s is. Binding Next.js's
`useSearchParams().get("q").unwrap_or_default()` showed it.

## Decision

**`unwrap_or_default()` of an `Option` whose `Some` isn't boxed is
`subject ?? default`, the subject as it's written:**

```js
return get(n) ?? "";
```

A boxed `Some` (ADR 0051), which `??` would give the box of, is read twice
still, kept first.

## Why

- **It's what JS writes**, and what `unwrap_or` already gives.
- **It's the same order**: the subject first, the default only where it's
  `None`, as Rust makes it.

## Consequences

- No corpus or snapshot output had one; react.dev's `PageHeading` did,
  `$splitBy(asPath, ..)[0] ?? ""` now, its `const option` gone.

## Amendment: `unwrap_or_else`

`unwrap_or_else(f)` of an `Option` whose `Some` isn't boxed is
`subject ?? f()` too, `f` called only where it's `None`, as Rust calls
it: react.dev's calculateNestedToc writes
`currentAncestors.get(item.depth - 1) || root`. A closure of statements
is its arrow, made first, which does nothing. A lowering test reads one
of each once, its fallback run only where it's needed; a mutation keeps
the `const option`.

## Amendment: a map of a literal's value and its fallback is one conditional

React's constructClassInstance writes `foundWillMountName !== null ?
`\n  ${foundWillMountName}` : ''`; its port, `found.map(|name|
format!("\n  {name}")).unwrap_or_default()`, printed `(foundWillMountName
!= null ? `\n  ${foundWillMountName}` : undefined) ?? ""`.

**A `map`'s `c ? v : undefined` of a value never `null` nor `undefined`,
a template's, a string's, a number's, a `bool`'s, an object's or an
array's literal, given its fallback, is `c ? v : fallback`**
(`option::or_fallback`). Case N ([0262](0262-when-rust-and-js-disagree.md)):
where `c` holds the value isn't nullish, so `??` keeps it; where it
doesn't, `??` gives the fallback.

Held by `test/lowering-option-truth.test.ts`'s "a map of a literal's
value and its fallback is one conditional", run, and the mutation
`map-fallback-coalesced` (`std_types/option.ts`).


## Consequence: an accepted port difference, th2b (2026-10-28)

React's createCapturedValueAtFiber writes `componentStack: stack !== null
? stack : ''`; its port's `stack.unwrap_or_default()`, clippy's
`manual_unwrap_or_default` form of the `if let`, is `stack ?? ""`. The
user accepted the difference by name (th2b): the same of a value that's
text or nullish. ADR 0262 case: None.
