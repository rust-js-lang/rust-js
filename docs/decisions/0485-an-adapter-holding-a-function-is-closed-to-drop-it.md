# 0485. An adapter that holds a function is closed to drop it

Status: Accepted. Extends [0465](0465-dropping-an-adapter-is-closing-it.md)
and [0470](0470-a-function-given-by-value-is-dropped-where-its-taker-drops-it.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's dropped, and
when, as std's.

## Context

A function given by value is dropped where its taker drops it (ADR 0470);
one a lazy adapter holds, `iter.map(move |x| x + factor.0)`'s, drops with
the adapter, and was refused: a `Map` of it was a std type holding a value
with a destructor. itertools' `MapSpecialCase::collect` and `counts_by` map
so, of a closure and of a generic `F`.

## Decision

**An adapter that holds a function with a destructor, `map`'s, `filter`'s
or `skip_while`'s, is made closing, as one holding an owning source is (ADR
0465)**: `$closing(it, (it) => { it.return(); .. })`, which closes its
source, then drops its function, as Rust drops a `Map`'s fields, its
`iter` then its `f`. It's dropped as any closed adapter is, `it.return()`,
when it's consumed by value (ADR 0464), dropped, or given to generic code.

- **Its function is held where it's given**, `recognition::held_function`,
  a question of types: given to it, it's neither dropped as the call
  returns nor refused (ADR 0470).
- **A closure that took a type parameter's value is dropped by that one's
  given drop**, as it's given one, a generic `F`'s too.
- **`take_while` and `inspect` of one**, lowered elsewhere, are refused
  still.

```js
const iter = $closing(
  tmp.map((item) => value(item)),
  (it) => {
    it.return();
    loudDrop_drop(factor);
  },
);
```

## Why

- **It's std's**: `held_function_drops` maps and filters with closures
  holding values with a destructor, collected, summed, stepped and dropped,
  in generic code too, with native Rust.

## Consequences

- **What compiles grows**: `map(f)`, `filter(p)` and `skip_while(p)` of a
  function holding a value with a destructor.
