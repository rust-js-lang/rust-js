# 0580. A call asks of a cell only what it may borrow

Status: Accepted. Extends [0328](0328-refcell-borrow-counts.md), whose
"nothing can ask" counted any call of the crate's as one that may, and
[0270](0270-module-variables.md)'s plain thread-locals.
Amended by [0863](0863-a-refcell-of-an-object-is-the-object.md): std's code
a call runs runs the crate's impls its bounds name. Extended by
[0897](0897-an-element-a-guard-reaches-is-its-place.md): an intrinsic runs
only what it's given, so `vec![..]` runs nothing. Extended by
[0898](0898-a-binding-that-runs-only-what-its-given.md): a binding that runs
only what it's given, given only numbers and text, borrows nothing.
Extended by [0907](0907-a-fn-pointer-runs-what-the-crate-made-it-of.md):
a fn pointer's call runs what the crate made one of its type of, not
anything, where no value of its type comes from outside. Amended
(2026-10-27): what JS runs borrows, of a thread-local only the crate's
code reaches, what the crate's bodies do but what its modules run as
they load; a `with_borrow` is held shared.

Case ([0262](0262-when-rust-and-js-disagree.md)): A. A borrow left
uncounted is one no other borrow of its cell can meet. A JS getter that
calls back into Rust and borrows a cell held, rare and harmless, isn't
seen (below).

## Context

React's scheduler moves each timer that's due from one heap to the
other:

```rust
TIMER_QUEUE.with_borrow_mut(|timer_queue| {
    while let Some(timer) = peek(timer_queue) {
        ..
        let timer = pop(timer_queue).expect("peeked");
        TASK_QUEUE.with_borrow_mut(|task_queue| push(task_queue, timer));
    }
});
```

A thread-local is its module's variable where each `with_borrow` closure
can't ask whether it's borrowed (ADRs 0270, 0328). ADR 0328 took a call of
any function of the crate's as one that may, so `peek`, `pop` and `push`,
which borrow no cell, made each queue a `{ value }` cell and each closure
`$withBorrowMut(timerQueue, ..)`, its borrow counted. Upstream's queues
are `var taskQueue = []`.

## Decision

**What each body of the crate may borrow is known, of its cells**
(`analysis/cell_borrows.rs`): a thread-local's by its key, a `RefCell`'s,
a `Mutex`'s or an `RwLock`'s by its type, each borrow shared or mutable.
A body borrows what it borrows itself, what each function of the crate's
and each closure it makes may, of the call's types, and anything where it
calls what rust-js can't see, a `dyn`'s method, a fn pointer, or a
binding that runs JS, or drops a value whose destructor runs (ADR 0320).
A type parameter's drop is its caller's: a summary keeps `T`'s, and a call
says whether `T` of it destructs.

**A borrow asks of one held only where it may be of its cell, and one of
them is mutable**: a thread-local's of its key, or of its cell's type;
two types alike. So:

- a `with_borrow` closure is quiet of its key where it borrows nothing
  that may be it: `TIMER_QUEUE`'s calls `TASK_QUEUE`'s and the heap's;
- a guard is momentary where nothing in its extent may borrow its cell
  (ADR 0328's `Quiet`, of the one question `call_asks`);
- a binding that reads a property, `node.sortIndex`, `in`, `instanceof`
  or `!!`, runs nothing, as one that makes an element or an object
  already did.

`may_ask_borrows` and `asks_no_borrows`, which answered "may this ask" of
any cell, are gone: `CellBorrows` answers it, of a cell.

```js
const taskQueue = [];
const timerQueue = [];
```

## Why

- **The question is of a cell**: a borrow of `TASK_QUEUE` can't fail
  for `TIMER_QUEUE` being held, and a function that borrows nothing can't
  fail any.
- **One owner**: what a call may borrow is `CellBorrows`', asked by the
  thread-locals and the momentary guards alike.

## Alternatives

- **Inline the crate's functions to see what they borrow**: what a
  summary says, per call. Rejected.
- **Trust every function of the crate's not to borrow**: a closure that
  borrows its own thread-local again would no longer throw where Rust
  panics. Rejected.

## Consequences

- No corpus case's JS changes; React's scheduler's queues are its arrays.
- A JS getter that calls back into Rust, borrowing a cell held where a
  property is read, isn't seen: its borrow passes where Rust's panics.
  No binding rust-js has does it.

## What holds it

- `test/lowering.test.ts`, "a thread-local whose closures call only what
  can't borrow it is its variable": two queues and the functions moving
  between them plain, and a closure that borrows its own key through a
  function still a cell, whose second borrow throws.
- `test/lowering.test.ts`, "a field's RefCell only borrowed for a moment
  is its value", and the corpus's `thread_local_borrow_inside`.
- Mutations: a key never its own, a function of the crate's borrowing
  nothing or anything, a body asked of nothing, an element's binding taken
  to run code, a call beside a guard taken as asking nothing.

## Amendment (2026-10-27): what JS runs borrows what the crate's code does

React's DEV dispatchers are upstream's module variables, set once where
the module loads, and each render sets the dispatcher to one:

```js
let HooksDispatcherOnMountInDEV = null;
if (__DEV__) {
  HooksDispatcherOnMountInDEV = { .. };
}
..
ReactSharedInternals.H = HooksDispatcherOnMountInDEV;
```

Ported, the variable is a thread-local set in `js::on_load!`, read by
`with_borrow(|dispatcher| set_h(dispatcher.as_deref()))`, where `set_h`
is a binding's call, which may run anything: so the borrow was counted,
the thread-local its `{ value: null }` cell, `$cellReplace($borrowMut(..))`
where it's set and `$withBorrow(.., (dispatcher) => { .. })` where it's
read. And a `with_borrow` was asked as a `with_borrow_mut`, mutably held,
so even a shared borrow elsewhere asked of it.

**What code rust-js can't see, JS's or a library's, may borrow of a
thread-local only the crate's code reaches is what the crate's bodies
borrow, but what its modules run as they load** (`cell_borrows.rs`'s
`anything_asks`): the union of every body's own borrows (`later`), but
`js::on_load!`'s own body, which runs once, as its module is evaluated,
before any of its functions can be called from JS, and whose closures
are bodies of their own. Only a thread-local of the crate's is so
(`closed_keys`): not `pub`, as another crate may borrow it, and each use a
read or a write of what it holds, `get`, `set`, `take`, `replace`,
`with_borrow` or `with_borrow_mut`, never its cell lent, `with`'s, which
code rust-js can't see may borrow by its type, nor its key handed on. Any
other cell is asked of as before, by anything.

**A `with_borrow` is held as it's held, shared** (`plain_locals.rs`):
only a mutable borrow asks of it. A `with_borrow_mut`'s, and a `with`'s,
which lends the cell, are held mutably, as before.

Case A ([0262](0262-when-rust-and-js-disagree.md)): a borrow left
uncounted is still one no other borrow of its cell can meet. JS can't
run a module's load again, and what it calls into is the crate's bodies.

```js
let HooksDispatcherOnMountInDEV = null;

if (__DEV__) {
  HooksDispatcherOnMountInDEV = { useId: () => 1 };
}

export function render(setDispatcher) {
  setDispatcher(HooksDispatcherOnMountInDEV);
}
```

- A borrow held mutably while JS runs, which may call a function that
  borrows the cell, stays checked: `$withBorrowMut(SEEN, ..)`, whose
  second borrow throws as Rust panics.
- A library's borrow of a cell the crate lends it by `with` isn't one of
  the crate's bodies', so a lent thread-local is asked of by anything, as
  before. No test of one crate can reach it; it's held by the rule's
  reading, not a mutation.

Held by `test/lowering-cell-borrows.test.ts`, "a thread-local set only
where its module loads is a variable JS can't meet a borrow of" (run, the
mutable one's second borrow throwing); mutations `unseen-runs-what-loads`,
`unseen-never-reaches-public`, `unseen-asks-everything`
(`analysis/cell_borrows.ts`) and `with-borrow-held-mutably`
(`analysis/plain_locals.ts`).

## Amendment: a cell an impl's method std runs borrows is of some type there (#1017)

The crate's `impl FromIterator for Sum` borrowing a `RefCell<I::IntoIter>`
of its own `I`, run by std's `collect`, whose args no type says (#240):
its borrow reached the body that called `collect` as it was, `I` and
all, and that body's run by `entry`, of its own args, put those in `I`'s
place, none, a rustc panic (`binder.rs`, "has parameters, but no args
were provided"). Only the crate's code reached from what it exports is
asked, so the case alone, with no `pub fn`, never panicked: the nets'
wrapper, `pub fn entry() { main() }`, is what reaches it. A borrow of a
run of no args is of a cell of some type there: each of the body's type
and const parameters a type nothing names, which `may_be_cell` takes for
any, as it takes a parameter. Held by the corpus case
`cell_of_an_iterator_std_runs`, in both builds, and the mutations
`cell-of-a-run-of-no-args-kept-its-parameters` and
`cell-of-a-run-of-no-args-kept-its-const-parameters`.

## Amendment: what a std call drops through a reference (2026-11-13, #1661)

What a body may borrow (`cell_borrows`' `direct`) took a body's drops to be
the values its own expressions have, but what's behind a reference, which
no body owns, and added nothing of what a std call drops. So
`g.borrow_mut().clear()` of a `Vec<Guard>`, whose `Guard`s' destructor
borrows a cell, was taken to drop nothing, the cell judged never to meet
another borrow, and its checks removed for the whole crate:
`try_borrow_mut` folded to `true`, and a `borrow_mut` there never panicked
`already borrowed`. `truncate`, `retain`, a map's `clear` and a
thread-local's `with_borrow_mut(|v| v.clear())` likewise.

What may run (`may_run`) had the rule, that a std call may drop a value of
each of its type arguments, as std's code owns, takes and replaces values
of them through whatever reference it's given. It's now one function,
`may_run::std_call_drops`, which both ask: `direct` gives each type it
yields the same check its expressions' types get. Case (ADR 0262): results
change, so Rust's checks are kept. Five snapshots keep checks they'd shed,
`owner_handle_quiet`, `rc_identity_set`, `refcell_identity_set`,
`refcell_transparent` and `set_for_each`, each through a std call given a
`&mut` of a type parameter, `ptr::hash(self, state: &mut H)` in the crate's
`Hash` impl, whose `H` is its caller's to say; their output is native's
either way. A narrower rule, none of a call given only shared references,
moved no snapshot and no case told it apart, so the one rule stays std's
broadest. Corpus cases `ref_cell_borrowed_by_a_clear_through_a_reference`,
`ref_cell_borrowed_by_a_truncate_retain_or_map_clear`,
`ref_cell_borrowed_by_a_thread_locals_clear` and
`ref_cell_borrowed_mutably_by_a_clear_through_a_reference`, against native
in both builds; a mutation, caught.

### Amended: what a std call drops is read from its signature (2026-11-13, #1661)

The rule above, every type argument, was std's broadest, and it cost a
whole port: the crate's `Hash` impl calls `ptr::hash(self, state: &mut
H)`, whose `H` became a type parameter's drop in its summary. Std's
`HashSet` code runs that impl with no type arguments given (#240), and a
summary with a parameter's drop left asks of every held cell, so every
guard held while a `HashSet<Rc<Fiber>>` was asked was counted:
react-in-rust's six files, 114 `$fieldCell`s and 60 `$cellReplace`s more
(ADR 0262: no observable difference, so JavaScript's form). `ptr::hash`
can't drop an `H`: it's lent one `&mut`, with no way to make another.

`may_run::std_call_drops(tcx, callee, args)` now reads the callee's
signature, parameter by parameter. Std's code may drop a value of a
parameter it holds, inside what a `&mut` or by-value input holds:
`Vec::clear`'s `T` of a `&mut Vec<T>`, `HashSet::insert`'s, a closure it
takes. It may also drop what it makes of those by their bounds, `entries`'
`D` of its `I: IntoIterator<Item = D>`, and a value it's lent `&mut`
itself where it can make another, given one by value or bound by `Default`
or `Clone`, `clone_from`'s. Not what it's lent shared, nor a type argument
no input holds. Still the one rule, what may run's and what a body may
borrow's. The port's JS is its committed JS again, and the five snapshots
above are theirs before #1661.

The lent half no program tells apart today, as every way a body reaches a
`&mut P` already counts `P`'s drop by a coarser path: a guard's
`deref_mut`, whose `T` the walk takes as held (a `RefMut` refers to its
`T`, #1680), an `FnOnce(&mut T)` bound's argument, or the `&mut` argument
at the call. It stays, as the rule is sound only with it, and its mutation
waits for #1680. Corpus cases `ref_cell_borrowed_by_a_clone_froms_drop` and
`printing_builder_written_step_by_step` (as a library), against native;
mutations `std-call-drops-every-argument` (the broad rule, caught by
`refcell_identity_set`'s snapshot), `std-call-drops-nothing-owned` and
`std-call-drops-nothing-made-by-bounds`, caught; `param-bound-never-std`
and `place-behind-ref-dropped`, which the broad rule let survive, caught
again.
