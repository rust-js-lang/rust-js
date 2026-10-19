# 0469. A function std leaves unrun is dropped there

Status: Accepted. Extends [0179](0179-option-filter-map-or-drops.md),
[0098](0098-destructors.md) and [0190](0190-generic-no-destructor.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's dropped, and
when, as std's.

## Context

A closure given where an `FnOnce` goes, `opt.map(move |x| x + a.get())`,
drops what it holds as its body ends, as Rust's does. But one std leaves
unrun, `map`'s of a `None` or `unwrap_or_else`'s of a `Some`, Rust's std
drops unrun, with what it holds, and JS dropped nothing: `a`'s destructor
never ran. So for generic code that gave its `F` to one, `value
.unwrap_or_else(f)`, of a closure its caller made. `ok_or_else` refused
such a function instead, a second answer to the same question.

## Decision

**Each function an `Option`'s, a `Result`'s or a `bool`'s method leaves
unrun is dropped where it's left**: `combinators::unrun_functions` tables
which argument each leaves on which variant, `map`'s on a `None`,
`unwrap_or_else`'s on a `Some`, `map_or_else`'s two each on one, `then`'s
on `false`. The call's subject and its function are held in variables, and
once the call's done, each is dropped where its variant says:

```js
const value = option != null ? f(option) : undefined;
if (option == null) {
  loudDrop_drop(a);
}
```

- **Of a type parameter, its caller's drop**, `dropF?.(f)`, which the
  generic function is given; one it isn't given, its callers give none with
  a destructor (ADR 0190).
- **What writes a place, or a map's entry, drops it where it branches**:
  `get_or_insert_with`'s `else`, and `$orInsertWith(map, key, make, dropKey,
  dropMake)`, after the key it doesn't keep, as std drops them.
- **One answer**: `ok_or_else` is in the table, not refused; `or_else` and
  `then` are given what they couldn't be.

## Why

- **It's std's**: `unrun_function_drops` runs each, with each variant, and
  generic code given a closure, with native Rust.
- **One place says what's left unrun**, and one says whether what's left is
  dropped (`drops_unrun`), which the inline combinators, `get_or_insert_with`
  and `or_insert_with` each ask.

## Consequences

- **What was silently left undropped is dropped**: a closure holding a value
  with a destructor that a combinator didn't run.
- **What compiles grows**: `ok_or_else`, `or_else` and `then` of a function
  that holds a value with a destructor.
