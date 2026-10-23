# 0635. A `filter` by truth, then a fallback, is `||`

Status: Accepted. Amends [0266](0266-text-or.md), whose `||` of a value
kept where it's truthy preparation writes now, of every fallback, a
chain's and a nullable result's too. Extended by
[0646](0646-a-held-chain-and-its-fallback-is-one-or.md): a chain held in a
`const`, then its fallback, is one `||` too.

Case: A ([0262](0262-when-rust-and-js-disagree.md)): no program can tell.

## Context

React falls back on any falsy value with JS's `||` everywhere:

```js
return type.displayName || type.name || null;
const functionName = innerType.displayName || innerType.name || '';
return getComponentNameFromType(type.type) || 'Memo';
```

The user decided (2026-10-23) a port writes std's own combinators, the
truth of a JS value `js::truthy`, the js crate's `!!`, and rust-js writes
them as `||`:

```rust
js::get(t, "displayName")
    .filter(|v| js::truthy(Some(*v)))
    .or_else(|| js::get(t, "name"))
    .filter(|v| js::truthy(Some(*v)))
```

rust-js wrote each `filter` as a conditional in a `const`, and `or_else`
as `??` of it, a nullable result's `?? null` after:

```js
const option = t.displayName ? t.displayName : undefined;
const option$1 = option ?? t.name;
return option$1 || null;
```

Tidy turned `(x ? x : undefined) ?? y` into `x || y` (ADR 0266), but only
where the conditional was read in place, which `or` and `or_else`, which
kept their `Option` in a `const`, and a `filter` of what isn't read the
same twice, a call's, never left it.

## Decision

**A `filter` whose predicate is the truth of what it holds, then a
fallback, is JS's `||`**: `x || y` gives `x` where it's truthy, else `y`;
the filtered `Option` gives `x` where `js::truthy(x)`, else what follows.
`filter` of `None` is `None`, and `undefined` is falsy, so they agree
there too.

```js
return t.displayName || t.name || null;
return inner.displayName || inner.name || "";
return componentName(t.type) || "Memo";
```

- **`filter` decides that its predicate is the truth** (`tests_its_truth`):
  applied to what it holds, it's `!!x`, `js::truthy(Some(*v))` and its
  forms of the closure, `|&v|`, or a `bool`'s own, `|b| *b`. Then its
  `Option` is `x || undefined`, `x` read once, with no `const`. Anything
  else, `!js::truthy(..)`, the truth of a part, `js::truthy(js::get(v,
  "name"))`, is its test, as before. A boxed `Some` (ADR 0051) is a box,
  whose truth isn't its value's: not this.
- **Each fallback reads its `Option` once**: `or_else`'s, and `or`'s of a
  `b` that does nothing, `o ?? b`, with no `const` of `o`; an `or` of a `b`
  that does something keeps `o` first, as Rust works out `o` before `b`.
  Of what a `filter` kept, each fallback takes its test and value in
  place, `unwrap_or`'s as before and now `or`'s, `or_else`'s,
  `unwrap_or_else`'s and `unwrap_or_default`'s: `a != null && !a ? a : b`
  (`option::or_fallback`).
- **Preparation writes the `||`** (`fallen_to`), the one place, of every
  shape lowering and tidy leave: `(x || .. || undefined) ?? y` is `x || ..
  || y`, as `x` where it's truthy is never `null` or `undefined`, and
  `undefined ?? y` is `y`; a chain that ends in another literal, `a ||
  "Memo"`, never needs `?? y`, a nullable result's `?? null`. And `a ? a :
  undefined`, a test tidied to the truth of what it gives, text's `x !=
  null && x !== ""`, is `a || undefined` too. So `a.filter(t).or_else(||
  b.filter(t)).or_else(|| c)` is `a || b || c`, and a `#[rust_js::nullable]`
  result's `a || b || null`, not `(a || b) ?? null`. A chain that ends in
  what may be `None`, `(a || b) ?? c` of `.or(b).or(c)`, keeps its `??`.
- **What else reads a filter's `Option` reads `x || undefined` as it read
  the conditional**: `map` takes its test, `!!x`, and value, `x ?
  x.toUpperCase() : undefined`; a `let … else` of it is `if (!x)` (tidy's
  `kept_by_truth`).
- **A test that's a `filter`'s is never a presence test**: `map` wrote
  `!!o ? o.m() : undefined` as `o?.m()`, where `!!o` is `o != null` of an
  `Option` never falsy (ADR 0298), and so gave `""` for an empty string
  `filter` had dropped. A `filter`'s test is kept as it is.

### Of a type whose JS is never falsy

Nothing is folded by type. `||` tests the very JS value `js::truthy` is
given, the item itself, so the two agree of every type: an object
reference, an array or a function is always truthy, so `x || y` is `x ??
y` there, as the filter keeps everything; text is falsy only where it's
empty, a number where it's `0` or `NaN`, a `bigint` where it's `0n`, all
as `!!` says. A `&Unknown` may be any of these, so it's the truth `||`
reads. Which a predicate is decides it, never what it's of.

## Why

- **No program can tell** (case A): each `||` gives what the `Option` the
  combinators make gives, `Some(x)` as `x` and `None` as `undefined`, or
  `null` where the result is nullable. It reads each value once, where the
  conditional read it twice.
- **It's the JS React writes.**
- **One owner each**: whether a predicate is the truth is `filter`'s;
  what a filter's fallback gives, `or_fallback`; that a fallback of what's
  truthy or `undefined` is `||`, preparation's, which sees every shape
  last. Tidy's fold of `(x ? x : undefined) ?? y` went, preparation's
  covering it.
- **It's tested**: `test/lowering-options.test.ts`, "a filter by truth then
  a fallback is JS's `||`", has the three React lines, `or`, `unwrap_or`, a
  chain, `or(b).or(c)`, a nullable `|| null` and `|| "Memo"`, a `map` after,
  and predicates that aren't the truth, and runs each against `""`, `0`,
  `undefined`, `null`, text and an object; mutations take each rule away.

## Alternatives

- **Tidy's fold alone**, made to see through the `const`s: it can't tell a
  `const` lowering spilled, read twice in the conditional, from one the
  program named, and the `||` would wait on the inlining of each.
- **A `filter` by truth as `x ? x : undefined`**: it reads `x` twice, a
  call's in a `const` first.
