# 0626. What a library hands a crate is what its items reach

Status: Accepted. Amends [0521](0521-a-dyn-fn-carries-its-drop.md) and
[0523](0523-libraries-share-one-counted-rc.md), whose "a crate using a
library" this narrows to what it uses of it. Amended: a closure of the
crate's a library's function is given hands nothing, below.

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
