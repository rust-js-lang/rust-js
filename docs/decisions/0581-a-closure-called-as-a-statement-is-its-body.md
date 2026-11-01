# 0581. A closure called as a statement where it's made is its body

Status: Accepted. Extends [0270](0270-module-variables.md), whose plain
thread-local's `with_borrow` is its closure given the variable, and
[0444](0444-a-comment-is-where-its-code-is.md)'s comments. Extended by
[0840](0840-a-thread-locals-with-is-its-closures-body.md): in what a
module runs when it's loaded too, and as what a statement evaluates first. Amended:
in a closure a constant's value makes too.

Case ([0262](0262-when-rust-and-js-disagree.md)): A. Each rewrite
changes nothing that runs.

## Context

A plain thread-local's `with_borrow_mut(|queue| ..)` is its closure
called with the variable (ADR 0270), as ADR 0580 now makes React's
scheduler's queues:

```js
function advanceTimers(currentTime) {
  ((timerQueue) => {
    while (true) {
      const timer = peek(timerQueue);
      ..
          ((taskQueue) => {
            push(taskQueue, timer);
          })(taskQueue);
```

`calls::apply` made a closure's value of a call where its body is one
`return`; one of statements stayed called where it was made. Upstream's
reads the queue it has: `push(taskQueue, timer)`.

## Decision

**A closure called as a statement where it's made, its value unread, of
a variable or a path that reads the same, is its body in its place**
(`js::applied_blocks`), its parameter read as what it's given. Not:

- where the body's own declarations, at its top, are another name of the
  function's, or the variable given;
- where its parameter is set, or a closure in its body could shadow it;
- where a `return` in it gives a value.

A `return;` in it, which leaves the closure, stays where the call ends the
function's body, and is the loop's `break;` where the body ends with a
loop it's in and in no other loop; else the closure stays.

**A comment before such a call is before its body**: what's between a
comment and the statement it's before may open a closure, `|queue| {`, as
well as start that statement (ADR 0444); and a blank line before a
statement is a line of nothing between, not a closure's opening line nor
an attribute's.

```js
function advanceTimers(currentTime) {
  // Check for tasks that are no longer delayed and add them to the queue.
  while (true) {
    const timer = peek(timerQueue);
```

## Why

- **It's what the program does**: the closure is `with_borrow_mut`'s way
  to lend the variable, and in JS the variable is there to read.
- **Of JS, not of a construct**: what's rewritten is a call of an arrow
  made in place, wherever plain thread-locals make it; it runs where they
  are made plain, after lowering.

## Alternatives

- **Inline it in a labeled block**, `block: { .. break block; }`, for any
  `return;`: JS no one writes for a loop's exit. Rejected for the loop's
  own `break`.
- **Leave a closure's body to tidy**: tidy runs before the thread-local is
  known plain. Rejected.

## Consequences

- Six corpus cases and two snapshots read their thread-local in place;
  the playground's loses a blank line a doc comment's attribute made.

## What holds it

- `test/lowering.test.ts`, "a plain thread-local's closure called as a
  statement is its body": a `push`, a loop's `return;` its `break;`, one
  that ends the function kept, comments before each, run.
- Mutations: the closure kept, its `return;` not broken, a tail one
  refused, its parameter unread, the comment lost, a closure's opening a
  blank line.

## Amended (2026-10-27): in a closure a constant's value makes

React's DEV dispatchers are thread-locals of `if __DEV__ {
Some(Dispatcher { .. }) } else { None }`, each a `const` of a
conditional, whose closures set the dispatcher to another's for the
time a hook runs:

```rust
use_memo: |create, deps| {
    let prev_dispatcher = ReactSharedInternals::h();
    INVALID_NESTED_HOOKS_DISPATCHER_ON_MOUNT_IN_DEV.with(|dispatcher| set_h(dispatcher.as_ref()));
    ..
},
```

In a function that's `setH(invalidNested ?? null)`; in the `const`'s
closure it was `((dispatcher) => { setH(dispatcher ?? null);
})(invalidNested)`. The pass that puts a closure called where it's made
in its place, `applied_in_place` in `std_types/cell.rs`, ran on each
function's body and on what the module runs when it's loaded, never on a
`const`'s value.

**Each closure a constant's value makes, a module's `const`'s or a
namespace's, is a function's body here too** (`js::applied_closures`):
its own closures called where they're made are their bodies, a `return;`
ending it as it ends a function. The value itself is no body, so nothing
is put in its place. Case N: the same code runs, in the same order, and
reads the same variable.

```js
const onMount = __DEV__
  ? {
      useA() {
        setH(nested);
        h = (h + 10) >>> 0;
        return 1;
      },
    }
  : null;
```

Held by `test/lowering-module-variables.test.ts`, "a thread-local's with
in a closure of a constant's value is its body", run; the mutation
`constant-closure-with-applied-kept` (`std_types/cell.ts`).

## Amended (2026-10-28): a closure whose value the function returns (bx1)

Case ([0262](0262-when-rust-and-js-disagree.md)): A, nothing that runs
changes.

React's warnAboutUpdateOnNotYetMountedFiberInDEV writes `if
(didWarnStateUpdateForNotYetMountedComponent !== null) { if (..has(name))
{ return; } ..add(name); } else { .. = new Set([name]); }`. Its port's
`DID_WARN.with_borrow_mut(|did_warn| { match did_warn { Some(..) => { if
.. { return true; } .. } None => *did_warn = Some(..) } false })`, the
function's value, printed `return ((didWarn) => { .. })(didWarn);`, and
the module variable a box, `const didWarn = {};`, which the closure's
`*did_warn = ..` set through: a closure whose `return` gives a value
stayed called.

**A closure called where it's made, whose value the function's body ends
returning, `return ((d) => { .. })(x);`, is its body in place, each
`return` of it the function's** (`applied.rs`'s `returned_block`): what
the closure returns is what the function returns, and its end, which
gives `undefined`, is the function's, which gives it too. The same
conditions as a statement's hold of its parameter and what it declares.
With no closure left to set it through, the variable is no box: `let
didWarn;`, `didWarn = new Set([name]);`. The corpus's `unsubscribe` of
`rc_fn_ptr_eq_of_moved.rs` reads its `hooks` so, and a `Fn`'s `(x0) =>
((x) => { .. })(x0.value)` is `(x0) => { x0.value.n = .. }`.

Held by `test/corpus/returned_closure_body.rs`, against native Rust, and
`test/lowering-cells.test.ts`'s `capped`; mutation
`returned-closure-called` (`js/applied.ts`).

## Amendment: the blank line before such a closure (rd44)

React's EventRegistry.registerDirectEvent writes a blank line, then
`NAMES.with(|names| { dict::set(names, name, n); });`, whose body is in
its place, `names[name] = n;`. The blank line was lost: the statement's
span starts inside the closure, and the line before it, the closure's
opening, isn't blank. **A blank line before a statement is looked for
past a closure's opening, `NAMES.with(|names| {`, as past a macro's
(ADR 0621) and its attributes' (ADR 0606)** (`to_oxc::blank_before`,
`openings::closure_opening`). A closure that stays one starts a block of
its own, which starts with no blank line, so it takes none. Held by
`test/lowering-comments-items.test.ts` ("a blank line before a closure
whose body is in its place is before its body") and the mutation
`blank-before-closure-opening-unseen` (`to_oxc.ts`).

## Amended (2026-11-01): a closure called last in a `switch`'s case (rd378, rd379)

React's trackUsedThenable ends `match thenable.status().as_deref()` with
`_ => { ..; SUSPENSE_EXCEPTION.with(|exception| js::throw(..)) }`. As
`if` chains, the closure called at the function's end was its body,
`throw exception;`. But `switches` (ADR 0629), which runs later, makes
the chain a `switch`, and the closure was then a case's
`return ((exception) => { throw exception; })(SuspenseException);`,
because a call at the end was looked for in an `if`'s branches and not
in a `switch`'s cases. And an arm's `SEEN.with(|seen| { if .. { return; }
.. })` stayed a closure in any `switch`, its thread-local then a box the
closure reads, `{ value: 0 }`.

**A case whose end leaves its `switch` (one that doesn't fall through to
the next, which is written its `break;`, and the last) is, for a closure
called last in it, as an `if`'s branch is: where the `switch` ends the
function, the closure's statements are in its place and each `return;` is
the function's; elsewhere each `return;` is the case's `break;`, where
none is in a loop or `switch` of the closure's own, whose `break` would be
that one's** (`leaving_cases`, `tail_applied`, `returns_as_breaks`). A
case that falls through goes on to the next one, so its closure stays one.
No case ends with a call that falls through yet: a guarded arm's case
ends with its guard's `if`. Held by `test/lowering-throws.test.ts` ("a
with whose closure throws, a switch case's value, is its throw") and
`test/lowering-switch-ends.test.ts` ("a thread-local's with that ends a switch's
case is its body"), and the mutations `applied-tail-switch-unseen`,
`leaving-cases-last-unseen` and `applied-case-return-unbroken`
(`applied.ts`).

in a `switch`'s cases. **In a `switch` that ends the function, the
closure called last in its last case is that case's statements, as an
`if` branch's is. In any other case it is too, where those statements
still end the case; one that would go on would fall into the next case,
so it stays a closure** (`tail_applied`). No input found so far reaches
that refusal: a closure whose value is returned ends on each path, even
a `loop`. Held by `test/lowering-throws.test.ts` ("a with whose closure
throws, a switch case's value, is its throw") and the mutation
`applied-tail-switch-unseen` (`applied.ts`).

## Amendment: what it's given is read where its body reads it (rd355, 2026-11-01)

React DOM's setInitialProperties writes `// We listen to these events ..`
above `MEDIA_EVENT_TYPES.with(|media_event_types| { for .. in
media_event_types { .. } })`. Since ADR 1042 the loop is `for (let i = 0;
i < mediaEventTypes.length; i++)`, and the comment printed inside its test,
before `mediaEventTypes.length`: the parameter, read as what the call gives
it, took the call's place in the source, before the comment's end, and the
comment went to the first node past it. **Each read of the parameter in the
body in its place is where the body reads it**: what the call gives it, its
place in the source cleared (`js::applied`'s `in_place`), so the comment is
above the body's first statement, the `for`, as upstream writes it. Case
([0262](0262-when-rust-and-js-disagree.md)): N, a comment's place.

Held by `test/lowering-index-loops.test.ts` ("a comment above a loop over a
thread-local's array is above the for") and the mutation
`applied-argument-at-call` (`js/applied.ts`).
