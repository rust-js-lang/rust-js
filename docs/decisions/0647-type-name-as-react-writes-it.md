# 0647. What `is_defined` names is no read; a test kept by its truth is `&&`

Status: Accepted. Amends [0645](0645-is-defined-of-a-static-is-of-the-static.md),
whose static was read before its test, and extends
[0635](0635-a-filter-by-truth-then-a-fallback-is-or.md)'s `||`.

Case ([0262](0262-when-rust-and-js-disagree.md)): A for the `&&`, `?.` and
`flatten()` forms: each is the same value where it's read. The read
dropped is a fix: results changed, a throw where the host has no
`Symbol`, and Rust's meaning, which reads nothing there, is kept.

## Context

React's `typeName`, ported:

```rust
let has_to_string_tag = js::is_defined(js::symbol::to_string_tag);
let r#type = value
    .filter(|_| has_to_string_tag)
    .and_then(|value| js::get(value, js::symbol::to_string_tag))
    .filter(|tag| js::truthy(Some(*tag)))
    .or_else(|| value.and_then(|value| js::get(value, "constructor")).and_then(|c| js::get(c, "name")))
    .filter(|name| js::truthy(Some(*name)));
```

was

```js
const toStringTag = Symbol.toStringTag;
const hasToStringTag = typeof Symbol !== "undefined" && typeof Symbol.toStringTag !== "undefined";
const option = value != null && hasToStringTag ? value : undefined;
const type = (option != null ? option[Symbol.toStringTag] : undefined) || value?.constructor?.name || "Object";
```

The first line threw where the host has no `Symbol`, the case the test
is for: MIR copies the static's value into `is_defined`'s argument, and
the call may unwind, so the copy was written before it. The rest wasn't
what React writes, `(hasToStringTag && value[Symbol.toStringTag]) || ..`.

## Decision

- **What `js::is_defined` is given isn't read**: it names what's asked of,
  so the temporaries that hold its argument are dropped before its call
  (`unread_defined_operands`), as its `typeof` path is all it writes.
- **`and_then` of what `filter` kept is its `f` where the test holds**,
  `value != null && c ? value.name : undefined`, with no `const` of the
  filtered `Option`, as `map`'s is.
- **`a != null ? a[k] : undefined` is `a?.[k]`**, of a key as of a
  property.
- **In `a || b`, `a` is kept only where it's truthy**, so `t ? x :
  undefined` there is `t && x`; and of `x != null && .. && x[k]` whose
  other tests do nothing, `.. && x?.[k]`: falsy where `x` is `None`
  either way. Preparation's `kept_by_truth`, beside ADR 0635's
  `fallen_to`.
- **`flatten()` of `bool::then` of an `Option`** is `c ? x : undefined`:
  `$someValue` of `c ? $some(x) : undefined`, and of `$some(x)`, is `x`
  there, `None` `undefined` either way.

```js
const hasToStringTag = typeof Symbol !== "undefined" && typeof Symbol.toStringTag !== "undefined";
const type =
  (hasToStringTag && value?.[Symbol.toStringTag]) || value?.constructor?.name || "Object";
return type;
```

## Why

- **Rust's `is_defined` reads nothing**: a read before the test is the
  throw the test exists to avoid.
- **It's React's `typeName`**, the `?.` the `Option`'s.

## Consequences

- The `strings` and `iterators` snapshots' `option ? option[0] :
  undefined` is `option?.[0]`, and so in eight corpus cases, `slice?.[0]`,
  `d?.[0] ?? 0`; `nested_options`' `$someValue($some($some(undefined)))`
  is `$some(undefined)`, and `boxed_option_results`' `$someValue(r.TAG ===
  "Err" ? $some(r._0) : undefined)` is `r.TAG === "Err" ? r._0 :
  undefined`. Each runs as native Rust does.

## What holds it

- `test/lowering-options.test.ts`, "React's typeName reads as upstream and
  asks before it reads": run with `Symbol` deleted from the global scope,
  of a tag, a constructor's name, neither and `None`; a `filter` whose
  test counts its calls, called only of a value; `flatten()` of `then`.
- Mutations in `scripts/mutations/lower/mir/structure.ts`,
  `lower/combinators.ts`, `js/make.ts`, `prepare.ts` and
  `lower/std_types/option.ts`.
