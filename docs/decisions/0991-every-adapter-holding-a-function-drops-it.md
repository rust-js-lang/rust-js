# 0991. Every adapter that holds a function drops it

Status: Accepted. Extends [0485](0485-an-adapter-holding-a-function-is-closed-to-drop-it.md)
from `map` and `filter` to each adapter that holds a function. Found by
#382's triage of `inspect-holds-no-function`.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change. What a
closure held was never dropped where Rust drops it, and four forms were
valid Rust refused.

## Context

```rust
let n = Noisy(7);
let seen = v.iter().inspect(move |x| { let k = &n; println!("{x} {}", k.0); }).count();
```

Rust drops the `Inspect`, and the closure with `n`, as the statement ends:
`drop 7`. ADR 0485 made a `map` or a `filter` whose function holds a value
with a destructor `$closing(it, (it) => { it.return(); drop f })`, in
`mir_iter_method`'s `holding_function`. `held_function` also named
`inspect`'s and `skip_while`'s, so the refusal of a kept function let them
through. But these are combinators, which `std_calls` lowers, and nothing
there dropped the function: `drop 7` was never printed. `flat_map`'s
function wasn't refused either, and wasn't dropped. `filter_map`,
`take_while` and `map_while` were refused.

## Decision

`held_function`, the one list of what an adapter holds as its function,
names each adapter's: `map`, `filter`, `filter_map`, `take_while`,
`skip_while`, `map_while`, `inspect` and `flat_map` (whose `F` follows
what it flattens). `std_calls`' combinators close an adapter that holds
one with a destructor as `holding_function` does a map's. Both use
`closing_function`: its source is closed, then its function is dropped,
as Rust drops an adapter's fields.

`scan()` holds its state too, so one whose function holds a value with a
destructor stays refused. The diagnostics test "a function with a
destructor a lazy take_while holds" pinned the old refusal, and is
removed.

## Consequences

`test/corpus/adapter_functions_dropped.rs` checks each adapter against
native Rust, consumed to its end, stepped part of the way, and held to its
scope's end. The mutations are `combinator-function-undropped` and
`flat-map-function-unheld`; `inspect-holds-no-function` and
`held-function-unclosed` point at the case too.

The same commit removes `Rev` from `back_stepped_source`, which nothing
reached once a `rev()` made in place is read by its form (ADR 0987). The
corpus, the forms corpus, the snapshots and memchr's shipped test are
unchanged without it. Its mutation, `rev-not-back-stepped`, is removed
(#382).
