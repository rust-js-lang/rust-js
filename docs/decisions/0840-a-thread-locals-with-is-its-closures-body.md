# 0840. A thread-local's `with` is its closure's body, wherever it's called

Status: Accepted; amended (in a closure's body too). Extends [0581](0581-a-closure-called-as-a-statement-is-its-body.md),
whose closure called as a statement in a function is its body, to what a
module runs when it's loaded and to a closure whose value a statement
reads first.

Case ([0262](0262-when-rust-and-js-disagree.md)): A. Each rewrite runs
what the call ran, in the order it ran it; where it couldn't, a closure
that returns early, the call stays.

## Context

A thread-local's `with(f)` is `f` called with the thread-local
(`calls::apply`), which a closure of one `return` already was in place:
`ENABLE_GESTURE_TRANSITION.with(|&enabled| enabled)` is
`enableGestureTransition`. One of statements stayed a closure called where
it's made, in two places React's port meets:

```js
// ReactSharedInternalsClient, what it runs when it's loaded
(reactSharedInternals => {
  if (enableGestureTransition) {
    reactSharedInternals.G = null;
  }
  ..
})(ReactSharedInternals);

// ReactNoopUpdateQueue's warnNoop
if ((didWarn => { const option = didWarn[warningKey]; return option != null && option; })(didWarnStateUpdateForUnmountedComponent)) {
```

Upstream writes `if (enableGestureTransition) { ReactSharedInternals.G =
null; }` at the module's top and `if
(didWarnStateUpdateForUnmountedComponent[warningKey])`. ADR 0581's
`js::applied_blocks` ran on functions' bodies only, and only of a call
that's a statement whose value nothing reads.

## Decision

**A closure called where it's made, of a variable or a path that reads
the same, is its body in its place, wherever a statement runs it first**
(`js::applied_blocks`, the one owner, ADR 0581's):

- **As a statement**, ADR 0581's, now in what a module runs when it's
  loaded too, `js::on_load!`'s. A `return;` there is the closure's, which
  a module can't write, so one that ends it stays a closure
  (`applied_blocks(stmts, false)`; a function's body, whose end a
  `return;` is, `true`).
- **As what a statement evaluates first** (`value_of`): an `if`'s test,
  a `const`'s, a `let`'s or a variable's value, what's returned or
  thrown, a `switch`'s discriminant, or the first operand of what they
  evaluate, `CALL + bump(3)`, `!CALL`, `CALL.x`. Its body, which returns
  only at its end, `return v;`, is before the statement, and `v` in the
  call's place: nothing of the statement ran before the call did, and
  `v` is read where the call gave it.

```rust
js::on_load! {
    REACT_SHARED_INTERNALS.with(|react_shared_internals| {
        if ENABLE_GESTURE_TRANSITION.with(|&enabled| enabled) {
            react_shared_internals.g.set(None);
        }
    });
}

pub fn counted() -> f64 {
    let n = INTERNALS.with(|internals| {
        let before = internals.count.get();
        internals.count.set(before + 1.0);
        before
    });
    n * 2.0
}
```
```js
if (enableGestureTransition) {
  ReactSharedInternals.G = null;
}

export function counted() {
  const before = Internals.count;
  Internals.count = before + 1;
  const n = before;
  return n * 2;
}
```

Not, so the call stays:

- a closure that returns before its end, whose `return` gives the
  closure's value, not the function's: `const found = ((internals) => {
  if (..) { return internals.calls; } .. })(internals);`;
- where what its body declares is another name of the function, of the
  statement beside it, or the variable given; where its parameter is set,
  or a closure in it could shadow it (ADR 0581's);
- of a variable's assignment to anything but a variable, `o.p = CALL`,
  or an operator's that reads its target first, `x += CALL`, which JS
  evaluates before the call.

## Why

- **It's what runs**: the closure is `with`'s way to lend the
  thread-local, which in JS is there to read; its body run where the call
  was runs each statement where it ran.
- **One owner**: ADR 0581's rewrite of JS, asked of each function and of
  what each module runs when it's loaded, where plain thread-locals are
  known (`plain_statics`).

## Alternatives

- **Inline at lowering, `LocalWith`'s**: lowering can't see the
  function's other names, which a body's declarations mustn't be, and a
  plain thread-local's `with_borrow` makes the same call after tidy; two
  places for one rule. Rejected.
- **A labeled block for an early `return`**: no person writes it, and a
  JS label is never an accepted difference. Rejected; the closure stays.

## Consequences

- A body's value read through a `let`, `let n = X.with(|x| { ..; before
  })`, is `const n = before;` after it: the program's two names, as it
  names them.
- The guide's `click()` (Export Rust APIs) is `clickCount.value = ..;
  return clickCount.value;`.
- The readability baseline's `return;` is 89 where it was 88, 9.3 in
  10,000 lines: `thread_local_with_in_place`'s closure whose `return;`
  isn't its end, which stays one, as it must.

## What holds it

- `test/corpus/thread_local_with_in_place.rs`, against native Rust: a
  statement's, an `if`'s test, a value's first operand, one returning
  early and one whose `return;` isn't its end.
- `test/lowering-cells.test.ts`, "a thread-local's with is its closure's
  body where it's called": React's two shapes, what a module runs when
  it's loaded, one that returns early, run.
- Mutations: `applied-value-kept`, `applied-value-early-return`,
  `applied-value-operand-unseen` (`scripts/mutations/js/applied.ts`),
  `applied-loaded-kept`, `applied-loaded-tail-return`
  (`scripts/mutations/lower/std_types/cell.ts`).

## Amendment: what the module runs sets a thread-local too

A thread-local's cell, its module's `const X = { value: init }`, is its
value where nothing but its module reads it by its `value`, a `let` where
it's set (ADR 0270). What set it was counted in functions alone, so once
`js::on_load!`'s closure was its body in the module's statements, its
`SEEN.with(|s| s.set(..))` was `const seen = 0; seen = ..`, which throws
as the module loads (#201). `plain_statics` asks the module's statements
too, what they set and what they bind of its name: `let seen = 0;`.
`test/modules.test.ts`'s order of a module's items holds it, with
`statics-loaded-set-unseen`.

## Amendment: in a closure's body too

Case ([0262](0262-when-rust-and-js-disagree.md)): A, as above.

React's `startTransition` reports an async scope's rejection through
`REPORT_GLOBAL_ERROR.with(|&report_global_error| return_value.then(|_|
noop(), report_global_error))`, in its `catch_unwind`'s closure. It was
`((reportGlobalError) => { returnValue.then(.., reportGlobalError);
})(reportGlobalError);`, where the same call in a function's body is its
body: `js::applied_blocks` walked the statements of a function and of
what a module runs, not of the closures they make, and the `try` that
`catch_unwind`'s closure became took its body as it was.

**`js::applied_blocks` takes each closure a body makes as a body of its
own, whose end a `return;` is**: an arrow's, an async arrow's and a
function expression's, each in place as a function's is. A `|&f|`
pattern parameter is a closure of one name already, as rustc's MIR binds
it. Upstream's `returnValue.then(noop, reportGlobalError);` needs `|_|
noop()` written `noop` too, a function given where a closure goes.

Held by `test/lowering-cells.test.ts`, "a thread-local's with is its
closure's body where it's called", whose `deferred` closure sets a
thread-local's dictionary through `with(|&did_warn| ..)`, run; and the
mutation `applied-closures-skipped`.
