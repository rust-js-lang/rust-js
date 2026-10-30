# 0959. `Rc::ptr_eq` of an `Rc<dyn Fn>` made of a closure made with it is `===`, uncounted

Status: Accepted. Extends [0582](0582-ptr-eq-of-an-object-is-its-identity.md),
`Rc::ptr_eq` of an object is its identity, to a `dyn Fn`'s function, and
[0852](0852-each-types-rcs-are-counted-apart.md), which types' `Rc`s are
counted. Amended: a closure neither `Copy` nor cloned is its own function wherever it's made (below).

Case ([0262](0262-when-rust-and-js-disagree.md)): N where every `Rc` that
may be the one compared is made of a closure written in its `Rc::new`: each
is a JS function made there, so two are `===` exactly when the `Rc`s are
one allocation. Elsewhere C: the `Rc`s are counted, as before.

## Context

React's test selectors keep commit hooks, and remove one by identity:

```rust
let commit_hook: Rc<dyn Fn()> = Rc::new(move || { .. });
COMMIT_HOOKS.with_borrow_mut(|commit_hooks| commit_hooks.push(Rc::clone(&commit_hook)));
// disconnect
let index = commit_hooks.iter().position(|hook| Rc::ptr_eq(hook, &commit_hook));
```

Upstream pushes the function and finds it by `indexOf`. `reads_rc_count`
took a `ptr_eq` of anything but an object to read a count (ADR 0320), so
every `Rc<dyn Fn>` of the crate was counted, `{ value() { .. }, strong: 1,
weak: 0 }`, with `$rcClone` and `$rcDrop`; the closure holding one,
`disconnect`, then dropped a value with a destructor, so it was `$dropping`
(ADR 0521), and every `dyn Fn` of the crate was taken to drop: the
reconciler's CacheComponent, Transition and RootScheduler, which compare
nothing, gained `$dropping`, `$dispose` and counted listeners.

A `dyn Fn` is the JS function itself (ADR 0022). Its identity is the
`Rc`'s where no two `Rc`s may be made of one function: two `Rc::new(hello)`s
of a function item are `hello === hello`, `true`, where Rust's two
allocations are apart; so are two of one `Copy` closure's variable, or a
type parameter's function given one item twice.

## Decision

**`Rc::ptr_eq` of an `Rc<dyn Fn>` is `===` of the functions, and counts
nothing, where every `Rc` the crate makes another `Rc<dyn Fn>` of that may
be that type is made of a closure given `Rc::new` as it's written,
`Rc::new(move || ..)`** (`rc_counts::counted_rcs`): a JS function made
there, which nothing else is. An `Rc<dyn Fn>` made of another, as a push
changes its lifetime, is that one, which its own making answers for.
Otherwise it's counted, as before: one made of a function item, a closure
given by its variable, a type parameter's function, a closure of another
crate's, or by `Rc::new_cyclic`.

- **Every `dyn Fn` may be every other** to `may_be` (ADR 0852), so one made
  apart counts every `Rc<dyn Fn>` the crate compares.
- **A library's `Rc<dyn Fn>`s are counted anyway**: a library's `dyn Fn`s
  may drop, its callers' closures unseen (ADR 0521), so the last owner of
  one runs a destructor, and its consumer counts what it hands it (ADR
  0852). A consumer's `Rc::new(hello)` given to a library that compares
  them is compared by its box (`test/crates/pairs/rc_fn_identity`).
- `recognition::compares_fn_identity` says which call asks; `is_rc_new`
  which call makes one.

## Why

- **It's the same program**: `rc_fn_ptr_eq` removes hooks by identity and
  compares hooks made in a loop of a `Copy` closure, against native Rust;
  `rc_fn_ptr_eq_of_item`, `_of_copy` and `_of_param` compare `Rc`s of one
  function, apart, as Rust's are.
- **Counting is for what only counts answer** (ADR 0582): the identity of
  a function made with its `Rc` isn't one.

## Alternatives

- **Count every compared `Rc<dyn Fn>`**, as before: the commit hooks' and
  every `dyn Fn`'s noise crate-wide. Rejected.
- **A fresh function wherever an `Rc<dyn Fn>` is made of one made apart**,
  `() => hello()`: wrong for two `Rc<dyn Fn>`s made of one `Rc<fn>`'s
  clones, which are one allocation. Rejected.

## What holds it

- `test/corpus/rc_fn_ptr_eq.rs`, `rc_fn_ptr_eq_of_item.rs`,
  `rc_fn_ptr_eq_of_copy.rs`, `rc_fn_ptr_eq_of_param.rs`, against native
  Rust; the pair `rc_fn_identity`.
- Mutations: `fn-identity-read-as-count`, `rc-new-of-closure-unseen`
  (`scripts/mutations/lower/recognition/wrappers.ts`), `fn-identity-of-fn-items`,
  `fn-identity-of-copied-closure`, `fn-identity-variable-made-where-given`,
  `fn-identity-through-dyn-counted` (`scripts/mutations/lower/analysis/rc_counts.ts`).

## Amendment: a closure neither `Copy` nor cloned is its own function wherever it's made (ts7, 2026-10-27)

ReactTestSelectors' `observeVisibleRects` compares its commit hooks by
`Rc::ptr_eq`. With it in the crate, ReactFiberHooks' `subscribe(Rc::new(
handle_store_change))`, the closure made in a block a `let` holds, was
"made apart" from its `Rc`, so every `Rc<dyn Fn()>` was counted, and with
it each `Rc` of what those hold, `UpdateQueue`, `Effect`, ..: 27 refusals
of a counted `Rc` given to JS.

What the rule guards is one JS function given `Rc::new` twice, two `Rc`s
Rust tells apart and JS doesn't. A closure value can be given twice only
where it's `Copy`, or cloned, a clone sharing its function. **A closure
neither `Copy` nor cloned is given `Rc::new` once, moved, so it's made
with its `Rc` wherever it was made** (`rc_counts.rs`, `made_with_it`):
`Copy` read where it's made an `Rc<dyn Fn>`, a clone from any body's
`Clone::clone` of its type.

- Case ([0262](0262-when-rust-and-js-disagree.md)): N, each `Rc` its
  own function.
- Held by the corpus cases `rc_fn_ptr_eq_of_moved.rs` (uncounted,
  compared by `===`), `rc_fn_ptr_eq_of_cloned.rs` (counted, `ptr_eq`
  `false` as natively) and `rc_fn_ptr_eq_of_copy.rs`; and the mutations
  `rc-fn-moved-made-apart`, `rc-fn-cloned-uncounted`,
  `rc-fn-copied-coerced-uncounted` and the amended
  `fn-identity-of-copied-closure`.
- ReactTestSelectors' held draft builds with Hooks in the crate; React's
  TestSelectors tests pass (37).

## Amendment: of a `&dyn Fn` made with it, `Rc::from(function::bind(..))` (rd231)

`function::bind` and `function::with_this` give the JS function itself,
`&'l dyn Fn(..)` (ADRs 0907, 0972, amended). React's hooks share a bound
dispatch and Rust closures alike, `let dispatch: Rc<Dispatch> =
Rc::from(function::bind(dispatch_set_state, ..))`: an `Rc<&dyn Fn>` unsized
to an `Rc<dyn Fn>`, which the hooks compare, `Rc::ptr_eq`. As one that may
share its function with another `Rc`, it was counted, every `Rc<dyn Fn>`
it may be with it; the closures holding them dropped what they held, and
`react_isomorphic`'s `dyn Fn`s, which drop nothing, refused the crate
(ADR 0626).

**An `Rc` of a `&dyn Fn` made right there of a function made right there,
`Rc::from(bind(..))` or `Rc::new(with_this(..))`, is that function, as one
of a closure made with it is:** no other `Rc` holds it, so `===` is
Rust's `ptr_eq` and nothing is counted. One made of a reference given it,
`let bound = bind(..); Rc::from(bound); Rc::from(bound)`, may be two
`Rc`s of one function, which Rust tells apart and JS wouldn't: counted,
as before.

Held by `test/cross-crate-items.test.ts`'s "an Rc of a bound function,
compared, makes no dyn Fn drop" (a library whose `dyn Fn`s drop nothing
given a closure holding one, run; and one reference made two `Rc`s,
`ptr_eq` false, as native Rust's), by react-in-rust's react-reconciler and
react-dom-bindings building, and mutations
`rc-of-bound-function-made-apart`, `rc-of-shared-reference-made-with`
(`analysis/rc_counts.ts`).
