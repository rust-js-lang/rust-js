# 0843. A `Weak` nothing upgrades is the value itself

Status: Accepted. Amends [0320](0320-counted-rcs.md), whose every `Weak`
counted the `Rc` it was made of; and fixes its counted `Weak::new()`, two
of which weren't `Weak::ptr_eq`.

Case ([0262](0262-when-rust-and-js-disagree.md)): A for a `Weak` nothing
asks whether it's alive; C for a counted one's `Weak::new()`.

## Context

React's context refers to itself: it's its own `Provider`, and its
`Consumer`'s `_context`. Upstream makes it, then sets them:

```js
const context = { $$typeof: REACT_CONTEXT_TYPE, /* .. */ Provider: null, Consumer: null };
context.Provider = context;
context.Consumer = { $$typeof: REACT_CONSUMER_TYPE, _context: context };
```

The Rust a person writes for that, as the Rust book teaches, holds the
references back as `Weak`s, so the `Rc`s make no cycle that never frees.
React's port had written `&'static` references of a `Box::leak`, which
the port rules forbid. But every `Weak` was counted (ADR 0320): its
`Rc` was `{ value, strong, weak }`, each `downgrade` a `$downgrade`, each
scope holding one a `try`/`finally` dropping it, and `Weak::new()` `{
strong: 0, weak: 0 }`. ADR 0320's own reason for counting a `Weak` is
that `upgrade` asks whether the value is alive, which only counts
answer.

And a counted `Weak::new()` was a new object each time, so
`Weak::ptr_eq(&Weak::new(), &Weak::new())` was `false`, where std's is
`true`: two `Weak`s to no allocation are equal.

## Decision

**A `Weak` of an `Rc` whose counts nothing reads is what that `Rc` is, the
value itself: `Rc::downgrade(&rc)` is `rc`, its clone is it, its drop does
nothing, and `Weak::new()`, and `Weak`'s `Default`, is `null`.** What asks a
`Weak` whether it's alive, `upgrade`, `strong_count` or `weak_count`, counts
its `Rc` as before (`reads_rc_count`), as `Rc::new_cyclic` still does.

| Rust | JS, uncounted |
|---|---|
| `Rc::downgrade(&context)` | `context` |
| `Weak::new()`, `Weak::default()`, a `Cell<Weak<T>>`'s `take()` | `null` |
| `weak.clone()` | `weak` |
| `Weak::ptr_eq(&a, &b)`, of a struct with fields | `a === b` |
| `Some(Weak::new())` | `$some(null)`, boxed, as `null` could be `None` |

```rust
let context = Rc::new(Context {
    // These are circular
    provider: Cell::new(Weak::new()),
    consumer: Cell::new(None),
});
context.provider.set(Rc::downgrade(&context));
context.consumer.set(Some(Rc::new(Consumer { context: Rc::downgrade(&context) })));
```

```js
const context = {
  // These are circular
  provider: null,
  consumer: null,
};
context.provider = context;
context.consumer = { context };
```

- **The one owner of counting**, `counted_rcs`, no longer counts a type
  for a `Weak` of it in a type: only for what reads a count. Lowering asks
  `counted_weak` and `uncounted_weak` (`std_types/rc.rs`), which answer it
  of a `Weak` as `counted_rc` does of an `Rc`; each place that read every
  `Weak` as counted, its clone, drop, `&mut`, copy and dictionary, reads
  `counted_weak`.
- **An uncounted `Weak` may be `null`**: an `Option` of one boxes its
  `Some` (`can_be_nullish`), and a `&mut` to one is a cell, as a
  `&mut` to a number is.
- **A counted `Weak::new()` is `$weakNone`**, the one `{ strong: 0, weak:
  0 }`, so two are `===`, as `Weak::ptr_eq` of std's are. It never
  upgrades; its weak count, which a clone and a drop change, is read as
  `0`, since nothing's strong (`$weakCount`).
- **A library, and a crate using one, counts every `Rc` and `Weak`**, as
  before (ADR 0523): its `Rc`s are shared with crates it can't see.

## Why

- **Nothing can tell**: a `Weak`'s count, or whether what it points at is
  alive, is all a `Weak` adds to its `Rc`, and only `upgrade` and the
  counts ask it; a drop that runs a destructor counts the `Rc` already
  (ADR 0320). Without them a `Weak` is an alias that can't be read, whose
  identity, `Weak::ptr_eq`, is the object's.
- **It's the JS a person writes**, and React's.
- **The idiomatic Rust**: `Weak` back-references, not `Box::leak`, nor
  `Rc` cycles, which never free.

## Alternatives

- **`Rc::new_cyclic(|weak| Context { provider: weak.clone(), .. })`
  printed as create-then-assign**: the closure's struct made with `null`
  where the `Weak` is read, then each such field assigned. It moves
  evaluation of those fields after the others, sound only where nothing
  between has an effect, and needs a lowering of its own; `new_cyclic`
  stays counted, `$newCyclic`, until a port needs it. Deferred.
- **Keep counting every `Weak`**: React's `createContext` is the counted
  `Rc`'s box, which React's renderers, JS reading `context._currentValue`,
  can't read. Rejected.

## Consequences

- `weak_debug`'s tree, whose `Weak`s nothing upgrades, is plain objects;
  its `Weak<i32>`, upgraded, is counted.
- `rc_counts`, `rc_dyn_listeners` and the others counting their `Rc`s
  read `$weakNone` where they made `{ strong: 0, weak: 0 }`.
- The readability baseline (`docs/readability.txt`) is blessed: the corpus
  is 126 lines shorter, `weak_debug`'s drops gone, so the shares of labeled
  blocks, `$iter` and `Iterator.from`, whose counts are the same, grew by
  0.1 in 10,000 lines; renamed variables fell by 48.

## What holds it

- `test/corpus/weak_cycle_set_after.rs`: React's context's cycle, with
  `Weak::new()`, `take`, `ptr_eq` of two to nothing and of two
  allocations, and `{:?}`, with native Rust, and as a library, counted.
- `test/corpus/weak_uncounted_uses.rs`: an `Option` of one, a derived
  clone of a struct holding one, one written through a `&mut`, replaced,
  taken, and `vec![weak; 3]`, with native Rust.
- Mutations in `scripts/mutations/lower/std_types/rc.ts`
  (`weak-every-counted`, `downgrade-uncounted-unknown`,
  `weak-new-uncounted-fresh`, `weak-new-counted-fresh`),
  `scripts/mutations/lower/recognition/wrappers.ts` (`downgrade-counts`,
  `weak-upgrade-uncounted`), `scripts/mutations/lower/representation.ts`
  (`uncounted-weak-never-nullish`), `scripts/mutations/lower/drops/types.ts`
  (`uncounted-weak-drops`) and `scripts/mutations/lower/std_impls.ts`
  (`uncounted-weak-cloned`).
