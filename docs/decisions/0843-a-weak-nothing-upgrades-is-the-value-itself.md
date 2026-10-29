# 0843. A `Weak` nothing upgrades is the value itself

Status: Accepted. Amends [0320](0320-counted-rcs.md), whose every `Weak`
counted the `Rc` it was made of; and fixes its counted `Weak::new()`, two
of which weren't `Weak::ptr_eq`. Amended below (the user's decision, rd96):
a closure's handle on itself is its own name.

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
  (Amended by [0844](0844-a-library-that-reads-no-count-counts-no-rc.md):
  one that reads no count counts none.)

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
  allocations, and `{:?}`, with native Rust, and as a library.
- `test/corpus/weak_debug.rs`: two counted `Weak::new()`s are
  `Weak::ptr_eq`, with native Rust.
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

## Amendment: a closure's handle on itself is its own name

The user's decision (rd96, 2026-10-29). Case
([0262](0262-when-rust-and-js-disagree.md)): A, no observable difference,
under the proof below; where it doesn't hold, counted as before.

React's createEventHandle makes a handle that adds itself to its target's
set of handles: `const eventHandle = (target, callback) => { ..
addEventHandleToTarget(target, eventHandle); .. }`. The Rust a person
writes for a closure that refers to itself is the Rust book's for any
value that does: its `Rc`'s `Weak` in a cell it captured, set right after,
and upgraded where it names itself:

```rust
let this_handle = Rc::new(RefCell::new(None));
let event_handle: Rc<ReactDOMEventHandle> = Rc::new({
    let this_handle = Rc::clone(&this_handle);
    move |target, callback| {
        let event_handle = this_handle.borrow().as_ref().and_then(Weak::upgrade).unwrap();
        ..
    }
});
*this_handle.borrow_mut() = Some(Rc::downgrade(&event_handle));
```

Its `upgrade` counted every `Rc` of the handle's type: `{ value, strong,
weak }`, the cell a checked `RefCell`, a `try`/`finally` of each, and in
react-dom-bindings ADR 0626's refusal, as its `dyn Fn`s then held what
drops.

**An `upgrade` inside the closure whose own `Rc` the `Weak` was made of
always succeeds, and asks no count.** The closure runs only borrowed from
an `Rc` of it, so the strong count is one at least while it runs: an
`Rc<dyn Fn>` can't give its closure up by value, and borrowck keeps the
`Rc` it's called through. The cell is set before anything can call the
closure, and never again. So where nothing else counts the handle's `Rc`,
the cell is no JS, its making, its clone, its setting and its drop, and
the upgrade is the closure's own name:

```js
const eventHandle = (target, callback) => {
  ..
  if (!doesTargetHaveEventHandle(target, eventHandle)) {
    addEventHandleToTarget(target, eventHandle);
    ..
```

What proves it, one owner, `analysis/self_handles.rs`, read of each body's
MIR as borrowck reads it, the one it's lowered from:

- a variable `Rc::new(RefCell::new(None))` of an `Option<Weak<D>>`;
- used only to clone it for a closure, given `Rc::new` (made an `Rc<D>`),
  and to set it, `*cell.borrow_mut() = Some(Rc::downgrade(&handle))`, of
  that `Rc`'s variable, and dropped;
- nothing between the handle's `Rc` and the cell's setting but the steps
  of setting it, so nothing can call the closure before;
- in the closure, the cell read only as `cell.borrow().as_ref()
  .and_then(Weak::upgrade).unwrap()`, to a variable of its own.

`counted_rcs` asks it which upgrades ask no count (`asks_no_count`); once
counting settles, a handle whose `Rc` is counted anyway, its counts read
or its drop running a destructor, is dropped from the facts, and lowered
as written (`uncounted`). Lowering writes nothing of the found statements
and calls, and the `unwrap`'s variable is the handle's variable's name.
Anything else, the cell read apart, a borrow held across its setting,
the closure called before it's set, `expect` for `unwrap`, is counted, as
before.

`Rc::new_cyclic(|me: &Weak<T>| ..)`, the other form, can't hold the
closure itself: `T` is sized and the closure would have to name its own
type. It's a struct's, `Rc::new_cyclic(|me| Handle { f: Box::new(..) })`,
which is ADR 0843's create-then-assign alternative, still deferred.

- `test/corpus/self_handle.rs`: a countdown that calls itself through its
  handle and React's handle that adds itself to a list once, with native
  Rust; `self_handle_counted.rs` (its count read; its cell read before
  it's set), `self_handle_called_early.rs` (called before its cell is set,
  its `unwrap` panics), `self_handle_maker_borrows.rs` (its maker holds a
  borrow as it sets it, `borrow_mut` panics) and
  `self_handle_reads_mutably.rs` (it borrows its cell mutably as it
  upgrades it, `borrow` panics), each counted, with native Rust.
- React's ReactDOMEventHandle in react-dom-bindings, ADR 0626's refusal
  gone: `const eventHandle = (target, callback) => { .. }`.
- Mutations in `scripts/mutations/lower/analysis/self_handles.ts`,
  `rc_counts.ts` (`self-handle-upgrade-counts`), `lower/analysis.ts`
  (`self-handle-counted-lowered`) and `lower/mir/structure.ts`
  (`self-handle-own-unnamed`, `self-handle-cell-written`).
