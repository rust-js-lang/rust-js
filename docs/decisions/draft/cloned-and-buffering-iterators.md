# An iterator that's cloned, or holds items with a destructor, is rust-js's own

Status: Draft. Would extend [0465](../0465-dropping-an-adapter-is-closing-it.md),
[0485](../0485-an-adapter-holding-a-function-is-closed-to-drop-it.md),
[0429](../0429-a-double-ended-iterator-knows-its-back.md), and
[0493](../0493-a-zip-steps-from-its-back.md); amend
[0164](../0164-double-ended-iterators.md) and
[0181](../0181-iterator-clones.md).

Accepted in part: a clone of a `map` or a `filter`, made again of a clone
of its source and its function
([0525](../0525-a-cloned-map-is-made-again.md)), with one owner of which
types are cloned, `analysis::clones`; and a `filter` or `filter_map` of a
double-ended iterator stepped from its back
([0526](../0526-a-filter-steps-from-its-back.md)). Its analysis's first part, a kind of
adapter the crate's types hold made explicit, `Enumerate` first
([0542](../0542-a-held-adapter-keeps-its-source-and-state.md)). What follows is what's
left.

Case: C ([0262](../0262-when-rust-and-js-disagree.md)): closure effects,
iteration state, cloning, and destruction are Rust's. Native JS adapters
are an optimization only where those observations are preserved.

## Context

A JS iterator hides the state needed to clone its current position or
step from its back. A generator also cannot clean up captures merely by
calling `return()` before its first `next()`: its body has not started.
Buffering an entire chain changes when closures run and values drop.

These are limits of that representation, not of JavaScript. Rust's
`Map::next_back` asks its source for the last remaining item and then
calls the mapping closure. An explicit adapter can do the same.

## Decision

**Keep explicit state for a std iterator wherever Rust needs operations
the native JS representation cannot preserve.** The iterator owner
chooses once, before lowering, and passes that answer to every use.
Native adapters remain only where the analysis proves them sufficient.

The explicit adapters use the existing runtime iterator conventions.
They implement JS iteration for consumers, with Rust capabilities
available only when rustc proves the corresponding trait bounds:

| Operation | Contract |
|---|---|
| `next()` | One forward step; no eager evaluation |
| `back()` | One backward step, only for `DoubleEndedIterator` |
| `len()` | Exact remaining length, only for `ExactSizeIterator` |
| `clone()` | Rust's `Clone` of the current state and owned fields |
| `return()` | Release owned state once; safe even before the first step |
| `[Symbol.iterator]()` | Return the same iterator without copying state |

Runtime objects must support the JS helper methods lowering calls
through the established iterator prototype or its existing bridge.
Merely providing `next()` does not supply `.toArray()`. Rust's
`Iterator::count` keeps its existing lowering; do not assume a JS
`.count()` method exists. A bridge must retain Rust capabilities and
ownership rather than erase `back` or `clone` from a value later given
to generic Rust code.

### Representation and ownership

- **One analysis owns the choice.** In the iterator analysis domain,
  propagate requirements through sources, adapters, assignments, aliases,
  returns and generic calls to a fixed point. Cloning, backward stepping,
  and ownership of buffered items, state or closures can require explicit
  state independently of the yielded item's drop. Lowering consumes the
  result; it does not rediscover it from emitted helper names.
- **No midstream conversion.** Once a native iterator has hidden its
  state, wrapping it cannot recover a clone or its back. All uses of one
  live iterator agree on its representation before it is constructed.
  If the analysis cannot prove agreement, use explicit state throughout
  the connected uses, conservatively throughout generic code if needed.
- **Clone uses the concrete Rust impl.** Copy current front/back positions
  and clone each owned field with its Rust `Clone`, in its impl's order.
  Clone captured closure state, buffered items and the remaining owned
  items of a vector iterator; do not rerun the chain from its beginning.
  A clone that panics drops completed cloned fields and leaves the
  original in the state Rust leaves it.
- **A lent source remains its lender's.** Closing an owning adapter
  releases its source; closing one over `by_ref()` does not destroy the
  lender's iterator. Preserve [0468](../0468-a-lent-adapter-leaves-its-source-to-its-lender.md).

### Backward stepping

`$rev` swaps forward and backward stepping; it does not buffer a forward
traversal. A mapped source's `back()` calls its source's `back()` and then
the mapping closure. For example:

```rust
let values: Vec<_> = (1..4)
    .map(|x| {
        println!("{x}");
        x * 2
    })
    .rev()
    .collect();
```

The closure prints `3`, `2`, `1`, and `values` is `[6, 4, 2]`. Alternating
`next` and `next_back` consumes one shared remaining interval.

A filter searches backward using its predicate; zip follows Rust's
alignment and trimming rules, including drops of discarded items.
Only rustc-approved capabilities are implemented: `scan` does not become
double-ended just because its source is. Missing implementations remain
compiler refusals, not claims that faithful reversal is impossible.

### Buffering and cleanup

Each adapter's state machine follows the pinned std implementation.
`Peekable` distinguishes "not peeked", "peeked end", and "peeked item",
including an item whose JS representation is `undefined`. `Scan` owns
its state and closure; `FlatMap` may hold both front and back inner
iterators. A non-fused source can yield after `None`, so exhaustion is
not automatically permanent closure.

**Drop order is per adapter, never a universal "inner then outer".**
For the pinned std, `Map` declares source then function, `Peekable`
source then cached item, and `Scan` source, function, then state.
`FlatMap` requires inspecting its nested state and drop order rather than
guessing from its API. Record those orders in differential fixtures.

Release each still-owned field once, including unstarted and partially
consumed iterators. Clear ownership before invoking user drop code, and
use the existing unwind cleanup machinery so one panicking destructor
does not skip the remaining required cleanup. Yielded or moved items no
longer belong to the adapter. General double-panic behavior remains the
existing documented limitation; changing it belongs to
[the panic proposal](catch-unwind.md).

### Separate compilation

Every std iterator represented by a separately compiled rust-js library
uses the explicit protocol. A binary using such a library makes the same
choice for its std iterators. Bump `library::VERSION` to record that
contract, validate transitive dependencies, and reject old artifacts.
There is no list of iterator names whose absence could mean either
"unused" or "represented natively".

The iterator owner answers this policy once, as the `Rc` owner answers
[its stable library policy](../0523-libraries-share-one-counted-rc.md). Crate-defined
iterators continue to use their Rust trait implementations; generic calls
use existing dictionaries and bridges, not replacement std adapters.
Library boundaries do not authorize calling capabilities rustc rejects.

## Acceptance

Add corpus cases and mutations for:

- Clone after partial forward and backward consumption; interleave the
  original and clone. Include stateful closures and owned `Drop` values.
- Reverse a side-effecting map and filter; alternate ends; reverse unequal
  zips and check the dropped surplus. Cover exact-size and non-exact-size
  sources separately.
- Drop before first use, after peeking, after exhaustion, after an early
  exit, and after a callback or clone panics. Include closure/state drops
  when the yielded item itself needs no drop.
- Nested flat maps with front and back state, `by_ref()`, non-fused
  sources, and `Option` payloads that need boxing.
- Pass and return iterators through generic functions and two libraries,
  including callbacks and trait bounds. Verify the old ABI is rejected.

Mutations lose clone position, share cloned owned state, eagerly map
before reversing, conflate peek states, close a lender, skip unstarted
cleanup, or drop a moved item twice. Existing native-adapter snapshots
must stay readable for standalone programs that need no explicit state.
Measure bytes and allocation cost for library-backed iterator workloads.

## Alternatives and consequences

Materializing on clone or reversal changes effects and ownership.
Recreating a chain loses its current position. Using native JS adapters
for library values and converting at the call site loses hidden state.

Explicit state costs runtime code and may select more helpers in generic
or library-backed programs. The first implementation covers a named set
of adapters and rejects the rest. Do not promise a fixed count of six
helpers, remove limitation entries wholesale, or mark an entire crate
supported before its corresponding cases pass.
