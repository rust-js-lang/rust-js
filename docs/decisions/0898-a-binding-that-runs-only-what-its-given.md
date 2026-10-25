# 0898. A binding that runs only what it's given, given only numbers and text, borrows nothing

Status: Accepted. Extends [0867](0867-a-binding-that-runs-nothing-and-aggregate-error.md)
(a binding that runs nothing borrows nothing) and
[0580](0580-a-call-asks-of-a-cell-only-what-it-may-borrow.md) (what a
call may borrow) to a binding that reads what it's given.

Case ([0262](0262-when-rust-and-js-disagree.md)): A, as ADR 0867's mark
is. A borrow left unchecked is one no other can meet: a marked binding
given only numbers, `bool`s and text runs none of the crate's code while
it's held, so its count is 0 wherever it's checked. Given anything else,
an object whose `toString` may be the crate's, it's counted as before,
and panics `already borrowed` where Rust does. What the mark can't see,
a program that replaces the host's function with its own, is the
consequence below, the same as ADR 0867's.

## Context

React's ReactFiberLane computes a lane's expiration time while it holds
the root's table of them:

```js
export function markStarvedLanesAsExpired(root, currentTime) {
  ..
  const expirationTimes = root.expirationTimes;
  ..
      expirationTimes[index] = computeExpirationTime(lane, currentTime);
```

and `computeExpirationTime` may report a bug, `console.error('Should have
found matching lanes. This is a bug in React.')`. Its port holds `let mut
expiration_times = root.expiration_times.borrow_mut()` across the call. A
binding is JS no one can see, which may call the crate back (ADR 0863),
so the borrow was counted, `$borrowMut(root.expirationTimes, true)` in a
`try`/`finally`, and with it every borrow of a `RefCell<Vec<f64>>` in the
crate. `Math.random()` was the same.

Which bindings can run the crate's code? Three ways reach it:

1. **What it's given**: a closure it calls, and, of a value it reads, a
   getter, `toString` or `valueOf`, `Symbol.toPrimitive`, or a proxy's
   trap, any of which may be the crate's.
2. **What it was given before**, or what's registered with the host: a
   listener `el.click()` runs, `dispatchEvent`'s, a custom element's
   `connectedCallback`, an observer's callback, a getter defined on an
   object it reads.
3. **The host's function replaced**: `console.error = f` of the crate's
   `f`, React's own `disableLogs`, through `Object.defineProperties`.

So "a binding given no closure or object of the crate's runs none of its
code" isn't a rule of types: `el.click()` is given nothing and runs a
listener (2), as ADR 0867 found. Only the binding's author knows it reaches
nothing by (2). What it's given (1) is a type's to say: a number, a
`bool` and text are primitives, and JS reads them, `String(x)`, `+x`, `%s`
of a format, by no code of anyone's.

## Decision

**A binding that says it runs no code but its own and what it's given,
`#[rust_js::runs_only_given]`, given only what runs nothing as JS reads
it, borrows nothing** (`cell_borrows::given_what_runs_nothing`): of each
parameter, as the call's generic arguments make it, a number, a `bool`,
a `char`, text, `()`, and `Option`s, tuples, arrays, slices and `Vec`s of
them. Anything else, an object of the crate's, a JS value, a closure, a
type parameter's, may run the crate's code, and the call is what it was,
anything.

- **The js crate marks `Math`'s functions**, `math::random`, `max`,
  `min`, `round` and `sign`, which read numbers.
- **The webapi crate marks the console's logging operations**,
  `console.assert`, `debug`, `error`, `group`, `groupCollapsed`,
  `groupEnd`, `info`, `log` and `warn`, `webapi/generate.ts`'s
  `RUNS_ONLY_GIVEN`; not `console.trace`, which may run
  `Error.prepareStackTrace`, what's elsewhere (2), nor `table` and `dir`.
- **It's `runs_nothing`'s by what it reads**: `#[rust_js::runs_nothing]`
  says a binding runs nothing of what it's given either, as `new
  AggregateError(errors)` reads its array and not what's in it, so it's
  given any value and borrows nothing; `runs_only_given` reads what it's
  given, so a value it's given says.

```rust
use std::cell::RefCell;
use webapi::console;

pub struct Root {
    pub expiration_times: RefCell<Vec<f64>>,
}

fn compute_expiration_time(lane: u32, current_time: f64) -> f64 {
    if lane == 0 {
        console::error(("Should have found matching lanes. This is a bug in React.",));
        return -1.0;
    }
    current_time + 250.0
}

pub fn mark(root: &Root, index: usize, lane: u32, now: f64) {
    let mut expiration_times = root.expiration_times.borrow_mut();
    expiration_times[index] = compute_expiration_time(lane, now);
}
```

```js
export function mark(root, index, lane, now) {
  const expirationTimes = root.expirationTimes;
  const value = computeExpirationTime(lane, now);
  expirationTimes[$at(expirationTimes, index)] = value;
}
```

## Why

- **The check it drops can't fail**: while the borrow is held, what runs
  is the host's function, reading primitives, which never reaches the
  crate.
- **Types say what a value can run, the author what the function does**:
  each claim is made by the one who knows it, as `cannot_throw` (ADR 0301)
  and `runs_nothing` are.
- **Upstream's JS**, `const expirationTimes = root.expirationTimes`, no
  count kept of a borrow no other can meet.

## Alternatives

- **A binding given no closure and no object of the crate's runs
  nothing**, what was asked first: wrong of `click()`, (2), a check left
  out that Rust's program fails. Rejected again, as in ADR 0867.
- **`runs_nothing` of `console.error`**: wrong of `console.error("%s",
  x)` of an object whose `toString` borrows the cell, which Rust's would
  panic on, (1).
- **One mark, `runs_nothing`, weakened to read what it's given**:
  `AggregateError`'s errors are any value, so its borrow would be counted
  again, ADR 0867's case undone.
- **Which crate functions JS holds, and whether any may borrow the cell**,
  to cover (3) too: React gives JS hundreds, its scheduler's callbacks
  among them, which borrow the root, so no borrow of it would go
  unchecked, and the gap stays open for a case no program meets.

## Consequences

- `#[rust_js::runs_only_given]` is in the guide's table of a binding's
  attributes, beside `runs_nothing`.
- **A program that replaces a marked function with its own, (3), is
  trusted not to borrow what's held across it**: React's `disableLogs`
  replaces `console.error` with `disabledLog`, which borrows nothing. One
  that did, and was called while a borrow of the same cell is held, would
  find no `already borrowed` where Rust's would panic. It's the trust
  every mark of a binding takes, `cannot_throw`'s of a getter defined
  where a data property was; the guide says it.
- A crate's own binding may say it; saying it of one that reaches the
  crate by (2) leaves a check out, as a wrong `runs_nothing` would.

## What holds it

- `test/lowering-cell-borrows.test.ts`, "a borrow held while a binding
  given only numbers and text runs isn't counted": ReactFiberLane's
  `markStarvedLanesAsExpired` of `console.error` and `Math`'s functions,
  its table the array; and of an object given to `console.error`,
  counted, and `already borrowed` where its `toString` borrows the cell.
- Mutations in `scripts/mutations/lower/analysis/cell_borrows.ts`:
  `runs-only-given-ignored`, `given-objects-run-nothing`.
