# 0626. What a library hands a crate is what its items reach

Status: Accepted. Amends [0521](0521-a-dyn-fn-carries-its-drop.md) and
[0523](0523-libraries-share-one-counted-rc.md), whose "a crate using a
library" this narrows to what it uses of it. Amended: a closure of the
crate's a library's function is given hands nothing, below; and a
library's `dyn Fn` drops where a library makes one of what drops, below;
and a library's own `dyn Fn`s drop as a program's do, a consumer that would
hand it one that drops refused, below.

Case ([0262](0262-when-rust-and-js-disagree.md)): R where it was wrong,
a library's `dyn Fn` dropped as nothing; else none: a refusal lifted.

## Context

React's scheduler came to depend on `shared` for `__EXPERIMENTAL__`, a
`bool` binding, and was refused where it built before:

```rust
TASK_QUEUE.with_borrow_mut(|task_queue| push(task_queue, timer));
```

`rust-js does not support with_borrow_mut of a value with a destructor`.
A crate using any library took every `Rc` to be counted, as its library
may share one (ADR 0523), and a `dyn Fn` of a type parameter's to drop,
as a library's closure may be given (ADR 0521): so the `Rc<Task>` the
closure moves dropped. `shared` hands the scheduler neither. And the
other way, a `Box<dyn FnMut>` a library's function returns, of its own
closure holding a value with a destructor, wasn't taken to drop at all,
the crate making no `dyn` of it.

## Decision

**What a library can hand a crate is what the items of its that the
crate's bodies use reach** (`Foreign::reached`, one walk): of each type a
value has, a library's function's parameters and result and a library's
type's fields, through each other, not what the crate's own types hold.

- **Its `impl Fn`**, the one way its closure reaches the crate: a type
  parameter's `dyn Fn` may then drop (ADR 0521's `foreign_drops`).
- **A `dyn Fn` in what it exports**: the crate's `dyn Fn`s drop, as one
  may already hold a value with a destructor.
- **An `Rc` or `Weak` in what it exports**: the crate counts every `Rc`,
  as the library does (ADR 0523).

A library whose items the crate uses are plain, a `bool` static's, hands
it none of them.

## Why

- **A library gives a crate only what its items' types carry**: a value
  of none of these types can't hold one.
- **One question, one owner**: whether `Rc`s are shared and whether a
  library's function drops are both "what do the libraries hand this
  crate", answered by one walk.

## Alternatives

- **Every library may hand anything, as before**: React's scheduler
  refused for a `bool`.
- **A library's manifest says what it hands out**: a fact its consumers'
  own types, through its generic functions, still decide.

## What holds it

- `test/cargo-link.test.ts`, "a library gives a crate's dyn Fns a drop
  only through its own functions": a library of a `bool`, of an `impl
  FnMut` given through a type parameter, and of a `Box<dyn FnMut>`.
- `test/crates.test.ts`, three crates sharing an `Rc`.
- Mutations `library-closure-unreached`, `library-dyn-unreached`,
  `library-reached-any`, `library-rc-unreached`, and ADR 0523's.

## Amendment: a closure of the crate's a library's function is given hands nothing

Case ([0262](0262-when-rust-and-js-disagree.md)): none; what changes is
which representation is taken, each correct.

React's ReactFiberCacheComponent shims an `AbortController`, its
listeners an `Rc<RefCell<Vec<Rc<dyn Fn()>>>>`. Elsewhere in the crate a
closure is given to a library's binding, `Thenable::then(&self,
on_fulfill: impl FnOnce(Option<&Unknown>) + 'static)`, holding an
`Rc<RefCell<Vec<&dyn Fn(..)>>>`. The shim became `{ value, strong, weak
}`, `$rcClone`, `$rcDrop` and `$dropping`, and ReactFiberTransition's
`ReactSharedInternals.G`, a chain of `dyn Fn`s, `$dispose`s in `catch`es
that moved a comment of the program's into one. The walk of the
library's function's signature, instantiated as the crate calls it, went
into the closure's type, its captures among its generic arguments, and
found the `dyn Fn(..)` there as if the library exported it: every `dyn
Fn` of the crate dropped, and so every `Rc` of one was counted.

**What a closure of the crate's holds isn't what a library hands it**:
the walk of a library's signature or fields doesn't go into a closure,
an `async` closure or a coroutine the crate made (`Foreign::reach`'s
`parts`). The crate made it of what it sees, and decides its drops as of
any closure it makes (ADRs 0521, 0873). A library's own closure, its
`impl Fn`, is still `functions`, and a `dyn Fn` its declared signature or
fields name still `fn_dyns`.

What stays: whether a `dyn Fn` drops is one answer for the crate (ADR
0521's `fn_dyns_drop`), not one per `dyn Fn` type: where one of the
crate's closures that drops is made a `dyn Fn`, every `dyn Fn` does.

### What holds it

- `test/cross-crate-items.test.ts`, "a crate's closure given to a
  library's binding hands the crate nothing": the shim's listeners an
  array, no `strong` nor `$dropping`, its listeners run.
- Mutation `crate-closure-taken-for-the-librarys` (`library.rs`).

## Amendment: a library's `dyn Fn` drops where a library makes one of what drops

Case ([0262](0262-when-rust-and-js-disagree.md)): none; what changes is
which representation is taken, each correct.

The amendment above left React's port as it was: its
ReactFiberCacheComponent shim still `{ value, strong, weak }`,
`$rcClone`, `$rcDrop` and `$dropping`, and ReactFiberTransition's
`ReactSharedInternals.G` chain `$dispose`s and `cancel$0$live`. Two
things still made every `dyn Fn` of the reconciler drop:

- **`react` and `shared` hand it `dyn Fn()`s**, `useMemo`'s `create`,
  `enqueueTask`'s callback, which a library's `dyn Fn` "may already hold
  one" took to drop. A library's own `dyn Fn`s drop, as its callers'
  closures are unseen; but which of the program's do is the program's to
  know: the crates that make them, each of which sees its own coercions.
- **Its `chain_thenable_value<T>` boxes `move |resolve|
  listeners.borrow_mut().push(resolve)`**, a closure that holds only an
  `Rc` of its own, which ADR 0521's rule took for a type parameter's
  function, its type naming `T` among its parent's generic arguments.

**A library's contract says whether it makes a `dyn Fn` of a function
holding a value with a destructor, or is handed one so by its own
libraries, as a program would find it** (`makes_dropping_fn_dyns`, as
`makes_dropping_anys` says of a `dyn Any`, ADR 0691; a library a
compiler wrote before is taken to): **a `dyn Fn` a library hands the
crate drops only where one says so.** The library's own `dyn Fn`s still
drop, whatever it says, as before. And **a function made a `dyn Fn` is a
type parameter's only where it may be**: a type parameter's own, `F`, or
a closure that captures a value of one; not a closure of a generic
function's that captures none, which its captures answer for, nor a `dyn
Fn` made another, `&dyn Fn(S, A) -> S` given on as React's `useReducer`
gives its reducer, a function already, whose own coercion answers (ADR
0521, amended).

React's reconciler's shim is then the JS a person writes, its listeners
an array, and the `ReactSharedInternals.G` chain plain functions: `react`
and `shared` say they make no `dyn Fn` of what drops.

What still holds: whether a `dyn Fn` drops is one answer for the crate,
not one per `dyn Fn` type: where one of the crate's closures that drops
is made a `dyn Fn`, every `dyn Fn` does. Here none is.

### What holds it

- `test/cross-crate-items.test.ts`, "a library's dyn Fn drops in its
  consumer only where a library makes one of what drops": the library's
  contract says it makes none, the crate's shim and a generic function's
  boxed closure carry no drop, and they run; "a library's dyn Fn of what
  drops is dropped where its consumer drops it", whose contract says it
  does, its value dropped after its call as Rust drops it; and "a library
  drops a dyn Fn its consumer gives it, of what drops".
- Corpus `dyn_fn_generic_closure_drops`, against native Rust: a generic
  function's closures, of what they capture.
- Mutations `library-fn-dyns-always-drop`, `library-fn-dyns-never-drop`,
  `library-fn-dyns-drop-unless-made` (`analysis.rs`),
  `library-fn-dyns-unwritten` (`lower/library.rs`),
  `library-fn-dyns-unread` (`library.rs`), `generic-closure-holds-a-param`
  and `dyn-made-dyn-holds-a-param` (`analysis/closures.rs`).

## Amendment: a library's own `dyn Fn`s drop as a program's do, and a consumer that would hand it one that drops is refused

Case ([0262](0262-when-rust-and-js-disagree.md)): none where it compiles,
each `dyn Fn` dropped where Rust drops what it holds; D, refused, for a
crate whose `dyn Fn`s may hold what drops that uses a library whose drop
nothing. The user's decision is pending: this is the recommendation taken
(`decisions-pending.md`, rd57).

React DOM's bindings depend on the reconciler, which `rust-js build` then
compiles as a library (ADR 0360). "The library's own `dyn Fn`s still drop,
whatever it says": every `dyn Fn` of the reconciler dropped, so its
`Rc<dyn Fn()>`s, and the `UpdateQueue`s, `Effect`s and the cache shim
holding them, were counted, `{ value, strong, weak }`, and 27 places that
give them to JS (`js::unknown_of(Rc::clone(queue))`,
`Rc::from(function::bind(..))`) were refused (ADR 0844): built as the app
it compiled, none of them counted. `react` and `shared`, libraries too,
wrote a `$dispose` in a `catch` beside each call that may throw while a
`dyn Fn` is held, `enqueueTask`'s and `startTransition`'s.

**A library's `dyn Fn`s drop as a program's do: where it makes one of a
function holding a value with a destructor, or a library hands it one whose
contract says its own may** (`fn_dyns_drop`, which its contract writes as
`makes_dropping_fn_dyns`, as before). **A crate whose `dyn Fn`s drop, that
reaches a `dyn Fn` in the items of a library whose don't, is refused**:
that library would drop nothing of one the crate gives it ("this crate's
`dyn Fn`s may hold a value with a destructor, which their drop runs, but
`dep`, a rust-js library whose `dyn Fn`s drop nothing, holds one of them").
What the libraries hand a crate is now said of each library
(`Reached::fn_dyns_of`), so the crate's `dyn Fn`s drop by the libraries
whose do, and are refused by those whose don't. A closure of the crate's
given a library's generic function, an `impl Fn`, that holds what drops is
the crate's own instance's, or refused, as before (ADR 0914).

```rust
// dep, whose `dyn Fn`s drop nothing
pub fn call_boxed(f: Box<dyn Fn() -> u32>) -> u32 {
    f()
}

// its consumer: refused
pub fn run(n: u32) -> u32 {
    let loud = Loud(n);
    dep::call_boxed(Box::new(move || {
        let loud = &loud;
        loud.0 + 1
    }))
}
```

### Why

- **One rule for what a library shares with its consumers**, as ADR 0852
  has it for `Rc`s: the library's contract says what its values are, and
  a consumer that would make one otherwise is refused, rather than the
  library guessing for consumers it can't see, which here made every
  library's `dyn Fn`s drop and React's reconciler unusable as a
  dependency.
- **One answer for a crate's JS**: the reconciler's is what it was as the
  app, whichever package `rust-js build` is given.
- **Still correct**: a `dyn Fn` the crate gives a library is one of the
  crate's, whose drop the crate knows; where it may drop and the library
  wouldn't, the crate is refused.

### Alternatives

- **A library's `dyn Fn`s drop, as before**: correct, but React's
  reconciler can't be a dependency of React DOM.
- **A fact per `dyn Fn` type**: more precise, but the reconciler's are in
  its public signatures, every one a consumer may give.
- **A consumer compiles its own copy of a library's function it gives a
  `dyn Fn` that drops**, as it does of a generic function (ADR 0914): a
  function that keeps it, in a thread-local the library reads, would still
  be the library's.

### Consequences

- A program that compiled is refused: a crate boxing a closure of a
  `Loud` into a library's `call_boxed`, whose `dyn Fn`s drop nothing. A
  library that makes a `dyn Fn` of what drops, a `ticker` of a `Tick`,
  drops one it's given, as before.
- `react`'s and `shared`'s JS lose their `$dispose`s: `enqueueTask`'s
  `$unwrap(enqueueTaskImpl)(task)`.

### What holds it

- `test/cross-crate-items.test.ts`, "a library drops a consumer's dyn Fn
  where its own drop, and refuses one it would drop nothing of".
- Mutations `dropping-dyn-fn-given-unrefused` and
  `dropping-dyn-fn-library-refused` (`analysis.rs`), and
  `library-fn-dyns-always-drop`, `library-fn-dyns-never-drop`,
  `library-dyn-unreached` and `library-reached-any`, rewritten; the
  amendment above's `library-fn-dyns-drop-unless-made` went with the rule
  it held.
