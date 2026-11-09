# 0959. `Rc::ptr_eq` of an `Rc<dyn Fn>` made of a closure made with it is `===`, uncounted

Status: Accepted. Extends [0582](0582-ptr-eq-of-an-object-is-its-identity.md),
`Rc::ptr_eq` of an object is its identity, to a `dyn Fn`'s function, and
[0852](0852-each-types-rcs-are-counted-apart.md), which types' `Rc`s are
counted. Amended: a closure neither `Copy` nor cloned is its own function wherever it's made (below); and each `Box`, `Rc` and `Arc` made a `dyn Fn` is apart where the crate reads a `dyn Fn`'s address (#1053, below).

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

Held by `test/cross-crate-dyn-fns.test.ts`'s "an Rc of a bound function,
compared, makes no dyn Fn drop" (a library whose `dyn Fn`s drop nothing
given a closure holding one, run; and one reference made two `Rc`s,
`ptr_eq` false, as native Rust's), by react-in-rust's react-reconciler and
react-dom-bindings building, and mutations
`rc-of-bound-function-made-apart`, `rc-of-shared-reference-made-with`
(`analysis/rc_counts.ts`).

## Amendment: a closure made an `Rc`'s is its own function only where the crate compares one (rd313, rd420)

A closure an `Rc` was made of, `Rc::new(move |e, i| renderer.default_on_uncaught_error(e, i))`, kept its arrow everywhere, so `Rc::ptr_eq` could tell two of them apart, even in a crate that compares none. createReactNoop's error handlers, an `Rc<OnError>` since rd308, printed `(error, errorInfo) => { NoopRenderer.defaultOnUncaughtError(error, errorInfo); }` at six sites, where upstream passes `NoopRenderer.defaultOnUncaughtError`. React ART's three default handlers printed the same.

**A closure made an `Rc`'s keeps its arrow only where the crate compares an `Rc<dyn Fn>` it doesn't count.** `counted_rcs` already knows the pointees `Rc::ptr_eq` is asked of (`asked`). `Counted::compared_uncounted` says whether any of those is left uncounted, where an `Rc`'s identity is its JS function's. Elsewhere the closure is what ADR 0605's takers make it: the function it passes everything on to, or the one it calls ignoring what it's given, as a `Box`'s is. A counted `Rc`'s identity is its box's, whatever the function is. So a crate that compares none has no answer that changes. A crate that compares one keeps every such arrow, which is conservative: `Rc<dyn Fn()>` and `Rc<dyn Fn(u32)>` are all `dyn Fn` to `may_be`, as above. A library's contract needs nothing new. A consumer's `ptr_eq` of what a library hands it is of what the consumer counts (ADR 0852), and a library compares its own as its own `Counted` says.

A thunk, `Rc::new(move || close(&closing))`, stays an arrow, `() => close(response)`. ADR 0605's bind rule takes only a closure with parameters of its own, and the user kept that rule (option A, 2026-11-02): React Flight's Node client's `close.bind(null, response)` is an accepted port difference.

Case ([0262](0262-when-rust-and-js-disagree.md)): A where the crate compares none (nothing observes an uncounted `Rc`'s identity); otherwise as before.

Held by `test/namespaces.test.ts`'s "a closure that passes all it's given to a namespace's function through a value is that function" (an `Rc<dyn Fn>` field, `onError: renderer.discreteUpdates`, run), and corpus `rc_fn_ptr_eq_of_forwarding` (two `Rc::new(|| ping())` compared, `false` as native's, each its own arrow), with the other `rc_fn_ptr_eq*` cases unchanged. Mutations `compared-uncounted-always`, `compared-uncounted-never` (`analysis/rc_counts.ts`) and `counted-closure-ungated` (`passed_on.ts`).

## Amendment: what a library merges, its consumer compiles (#1052)

A library that compares no `Rc<dyn Fn>` counts none, so its `Rc::new(read.clone())`
and `Rc::new(read)` are one JS function. It also makes a closure an `Rc` is
made of the function it passes on to. Its consumer may compare either.
`counted_rcs` now says which functions make such an `Rc`, `Counted::merges`.
The lowering records a closure an `Rc` is made of that passes on. A
library's manifest lists both, `merges_identities`. A consumer that
compares a function's identity compiles its own of each (ADR 0914,
amended). Its `counted_rcs` reads what a body of MIR alone makes another
`Rc`'s, so that compile counts them as the crate's own would.
`identity_read` is the one test of what reads an identity. `counted_rcs`
asks it, and so does `reads_fn_identity`, which says whether a consumer
compiles them.

## Amendment: each allocation made a `dyn Fn` is apart where its address is read (#1053, the user's decision)

A `Box<dyn Fn>` of a `fn` pointer is the pointer itself, and an uncounted
`Rc<dyn Fn>` of a cloned closure is the closure (ADRs 0125, 0023), so two
made of one function were one JS function: `ptr::addr_eq(&*a, &*b)` of
`boxed(double)` twice, and of `Rc::new(read.clone())` and
`Rc::new(read)`, was `true`, where Rust's two allocations are apart,
`false`. Counting doesn't help: `&*rc` is still the function. Probes
found it of every kind of function another may hold too: a `fn` pointer
(`Box`, `Rc`), a function item (`Rc`, `Arc`), a `Copy` closure, a cloned
closure, a type parameter's, and `Rc::as_ptr` read the same.

**Where the crate reads a `dyn Fn`'s address (`points_at_functions`),
each `Box`, `Rc` and `Arc` made a `dyn Fn` of a function another may hold
too, what `made_with_it` doesn't say is its own, is a function of its own
that calls it, an arrow of the `dyn Fn`'s parameters, `(arg0) => f(arg0)`**,
as `Box::new(move |x| f(x))` prints (`rc_counts::counted_rcs`'
`Counted::apart`, `own_function`; `traits/dyns.rs`). `made_with_it` now
sees a closure written in its `Box::new` too. Only a `dyn Fn` whose
address may be read is: one of the arguments a raw pointer's `dyn Fn`
takes, which no coercion of a `dyn Fn` changes (`address_read`), so
React's crates, which read the address of one signature, keep
`unsuspend: noop` of another. Where it's made:

- **A `Box`'s, and an `Rc`'s of what's no closure, where it's unsized**:
  `function boxed(f) { return (arg0) => f(arg0); }`. A `fn` pointer read
  back out of an `Rc<fn>` is still the function, so `*rc == double` is
  still `true`. A counted `Rc`'s is its cell's, `{ value: (arg0) =>
  double(arg0), strong: 1, weak: 0 }`. One that drops what it holds
  carries the drop, `$dropping((..) => f(..), () => $dispose(f))`.
- **An `Rc`'s or an `Arc`'s closure, where it's given `Rc::new`**
  (`rc_new_apart`, `own_function_given`, `std_types/values.rs`): a
  closure has no identity but its address, and an `Rc` of one cloned
  before it's unsized is one allocation, which one function keeps.
- **An `Rc` of a `fn` pointer or item that may be shared before it's
  unsized, a clone of it or a `Weak`'s upgrade, is refused**
  (`shared_apart`): wrapped where it's unsized, one allocation would be
  two functions, and wrapped where it's made, a `fn` pointer read out of
  it would compare apart from its function. One unsized where it's made,
  `Rc::new(f)`'s, has no clone before (`made_where_unsized`).

**Across crates, as #1052 did for `Rc::ptr_eq`**: a library that reads no
address makes none apart, `boxed(f)` is `f`, so each of its functions
that makes a `Box`, `Rc` or `Arc` a `dyn Fn` of what another may hold too
is listed `merges_identities` (`Counted::merges`), and a consumer that
reads an identity compiles its own, whose MIR's coercions `counted_rcs`
reads, boxes too (`owners_made_others`): there each is apart. One a body
of MIR alone unsizes is taken as unsized elsewhere than where it's made.
A closure of the library's is its own there unless that body copies or
clones it (`its_own`, `clones_of`), as one of the crate's is, so
`computed`'s `Box::new(move |previous| ..)` stays its arrow. The one list
of what merges is now this, the `Rc<dyn Fn>`s #1052 listed among them,
counted or not: a counted `Rc`'s function is shared by its clones too.
Library ABI 22.

A crate that reads no `dyn Fn`'s address is unchanged: the corpus's JS is,
but the new cases', and React's port regenerates with no new difference.

The user chose each option (2026-11-09): the arrow over `f.bind(null)` or
a refusal; every allocation apart, a generic `Box<F>` too, where a
`Box` of a value of no size allocates nothing and native Rust's two
compare equal, which Rust promises nothing of, ADR 0262's second case,
listed in its Consequences and the guide's differences (48), with two
`make()`s of `Box::new(|x| x + 1)`, apart before; and the refusal above.

Case ([0262](0262-when-rust-and-js-disagree.md)): C, Rust's answer kept;
B for a value of no size.

Held by the corpus's `dyn_fn_addresses_of_boxes`, `dyn_fn_addresses_of_rcs`
and `dyn_fn_address_by_rc_as_ptr`, against native Rust, the refusals
`dyn_fn_address_of_a_shared_rc_refused` and
`dyn_fn_address_of_an_upgraded_rc_refused`, the pair
`library_allocations_of_one_function`, against native Rust, and
`test/lowering-fn-identity.test.ts`'s "two boxes of a value of no size"
pin. Mutations `allocations-an-instance-makes-unseen`,
`library-allocations-merge-unlisted`, `allocations-of-one-function-merged`,
`allocation-address-of-any-signature-read`, `library-closure-moved-once-apart`,
`library-closure-clones-unseen`, `library-closure-copies-unseen`,
`allocation-of-a-function-made-with-it-apart`,
`allocation-functions-apart-unread`, `box-new-closure-made-apart`,
`rc-closures-apart-where-unsized`, `shared-rc-of-a-fn-pointer-unrefused`,
`upgraded-rc-of-a-fn-pointer-unrefused`, `rc-made-where-unsized-shared`
(`analysis/rc_counts.ts`), `allocation-function-unwrapped`,
`shared-allocation-unrefused`, `counted-allocation-function-whole`,
`counted-held-function-read-late`, `allocation-function-drop-lost`,
`rc-new-function-drop-lost`, `own-function-of-no-arguments`
(`traits/dyns.ts`), `rc-new-closure-shared` and
`rc-new-closure-own-uncounted` (`std_types/values.ts`).

## Amendment: a copy of a closure lent as a `&dyn Fn` is a function of its own (#1090, the user's decision)

Case ([0262](0262-when-rust-and-js-disagree.md)): C, the results changed.
A closure's JS function is its place: `&c1 as &dyn Fn()` is `c1`, and two
pointers to it compare by the function's identity. A copy of a `Copy`
closure that captures something, `let c2 = c1;`, was the same function, so
`ptr::eq(&c1 as &dyn Fn() -> i32, &c2 as &dyn Fn() -> i32)` was `true`
where native Rust's is `false`: two places. A closure of no size is the
function, its address no allocation's, as before.

**Where the crate reads a `dyn Fn`'s address (`points_at_functions`), a
copy of a capturing `Copy` closure a reference to which it makes a `&dyn
Fn` or a pointer to one is a function of its own, `const c2 = c1.bind();`**,
a distinct function that runs as the original, an arrow's `this` being
none. Which closures: `Counted::copied_apart`, this decision's owner, of
each a body lends so (`lent_apart`), or any, where generic code lends a
type parameter's value so (`lent_any`). `copies.rs` copies one as what's
changed in place is, so a struct's field, a tuple's and each of `[c1;
3]`'s items is its own too. A `Box`'s or an `Rc`'s closure is no reference
lent: each is a function of its own where it's unsized already (#1053), so
`dyn_fn_addresses_of_boxes` and `dyn_fn_addresses_of_rcs` are as they
were. Every other program's copy is the function, unchanged (the user's
choice of the options: refusing the comparison, or `.bind()` of every
copy, were the others).

Held by the corpus cases `copied_closures_compared` and
`copied_closures_lent_to_generic_code`, against native Rust;
`test/lowering-fn-identity.test.ts`'s "a copy of a capturing closure is a
function of its own only where a dyn Fn's address is read", both forms;
mutations `copied-closure-shared` (`lower/copies.ts`), `lent-closure-unseen`,
`lent-param-unseen` and `lent-of-a-box` (`lower/analysis/rc_counts.ts`).
