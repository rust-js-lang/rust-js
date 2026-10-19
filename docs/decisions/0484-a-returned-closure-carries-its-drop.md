# 0484. A closure its maker gives back carries its drop

Status: Accepted. Extends [0098](0098-destructors.md) and
[0470](0470-a-function-given-by-value-is-dropped-where-its-taker-drops-it.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's dropped, and
when, as std's.

## Context

A closure that holds a value with a destructor drops it through the
variables it took, where it's made (ADR 0098): only its maker sees them. One
its maker gives back, the type its `impl Fn` hides, is dropped by its
caller, or by whom its caller gives it to: itertools' `key_to_cmp(key)`
gives `move |a, b| key(a).cmp(&key(b))` to `k_smallest_by`. It was refused
where it was made, and where it was dropped.

## Decision

**A closure its maker gives back carries its drop**:
`recognition::carries_its_drop`, a question of its type alone, which its
maker and every caller ask.

- **Made, it's `$dropping(f, drop)`**: `f`, carrying `drop`, which drops
  what it took, under JS's own `Symbol.dispose` (or the symbol registered
  for it where an engine has none, `$disposeKey`).
- **Dropped where its maker's variables aren't seen, it's `$dispose(f)`**,
  that drop run; given to generic code as its drop, `$dispose` itself.
- **One that leaves another way, a struct's field**, is refused still.

```js
function key_to_cmp(key, KOrd, dropF) {
  return $dropping(
    (a, b) => KOrd.cmp(key(a), key(b)),
    () => {
      dropF?.(key);
    },
  );
}

const value = smallest([5, 2, 9], cmp, { clone: (value) => value }, $dispose);
```

## Why

- **It's std's**: `returned_closure_drops` gives back a closure holding a
  generic `F` and one holding a value with a destructor, drops one where
  it's held and gives the other to generic code that drops it, with native
  Rust.
- **JS's own protocol**: what holds something to let go of has
  `[Symbol.dispose]`, which a `using` declaration calls.

## Consequences

- **What compiles grows**: a closure holding a value with a destructor
  given back by its maker, as itertools' `key_to_cmp` is.
