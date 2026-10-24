# 0830. What follows a `let Err(x)` of a closure that never returns is its `catch`

Status: Accepted, by the user's decision of 2026-10-24. Amends
[0706](0706-panic-is-the-one-way-a-port-throws-an-error.md), whose
`catch` of a `let Err(x) = catch_unwind(f)` was what follows up to its
last read of `x`, what that declared and the rest read a `let` before
the `try`; and [0660](0660-catch-unwind-catches-what-js-throws.md), whose
`try` it is.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. What runs, and in
what order, is Rust's: the `catch` always runs, so what it holds runs once
as it would after it, and nothing after it reads what it declares.

## Context

React's `describeBuiltInComponentFrame` reads the engine's prefix and
suffix off the stack of an error it throws:

```js
if (prefix === undefined) {
  // Extract the VM specific prefix used by each line.
  try {
    throw Error();
  } catch (x) {
    const match = x.stack.trim().match(/\n( *(at )?)/);
    prefix = (match && match[1]) || '';
    suffix = x.stack.indexOf('\n    at') > -1 ? ' (<anonymous>)' : ..;
  }
}
```

The port wrote `if let Err(x) = catch_unwind(|| panic!("")) { .. }`,
which rustc warns is irrefutable: the closure never returns, so its
`Result` is `Result<!, _>`, whose `Ok` is uninhabited. rustc's suggestion
is `let Err(x) = catch_unwind(|| panic!(""));` and the block's statements
after it, which rust-js printed as ADR 0706 decided:

```js
let stack;
try {
  throw new Error();
} catch (x) {
  stack = x.stack ?? "";
}
const match = /\n( *(at )?)/.exec(stack.trim());
prefix = match?.[1] ?? "";
suffix = ..;
```

## Decision

**Where `f` never returns, its `Ok` uninhabited, the `catch` of `let
Err(x) = catch_unwind(f)` holds the shortest run of the statements after
it, in its block, that holds every use of `x` and of every binding the run
declares, transitively; the rest of the block follows the `try`.**

```rust
if PREFIX.with_borrow(Option::is_none) {
    // Extract the VM specific prefix used by each line.
    let Err(x) = catch_unwind(|| panic!(""));
    let stack = x
        .downcast_ref::<&'static JsError>()
        .and_then(|x| x.stack())
        .unwrap_or_default();
    let r#match = js::reg_exp::new(r"\n( *(at )?)", "").exec(&js::string::trim(&stack));
    PREFIX.set(Some(r#match.and_then(|m| m.get(1)).unwrap_or_default()));
    SUFFIX.set(if stack.contains("\n    at") { " (<anonymous>)" } else { "" }.to_string());
}
```

```js
if (prefix == null) {
  // Extract the VM specific prefix used by each line.
  try {
    throw new Error();
  } catch (x) {
    const stack = x.stack ?? "";
    const match = /\n( *(at )?)/.exec(stack.trim());
    prefix = match?.[1] ?? "";
    suffix = stack.includes("\n    at") ? " (<anonymous>)" : "";
  }
}
```

React's `DetermineComponentFrameRoot` sets a variable its closure
captured, which nothing in the run declares, so its `catch` is that
assignment, and what follows reads neither:

```rust
let Err(x) = catch_unwind(|| panic!(""));
control = x.downcast_ref::<&'static JsError>().copied();
render();
```

```js
try {
  throw new Error();
} catch (x) {
  control = x;
}
render();
```

- **Only where `f` never returns.** The irrefutable `let Err(x)` is
  lowering's `const { _0: x } = $catchUnwind(f)`, which it writes only of
  the pattern rustc found irrefutable, an uninhabited `Ok`; preparation
  (`unwound_let`, `prepare/catches.rs`) makes it a `try` only where `f`'s
  body ends in a `throw` and returns nowhere, as ADR 0706 decided. An
  `if let Err(x)` of a function that may return, `catch_unwind(fake)` of
  a `fn fake() -> Option<..>`, tests the `Result`'s `TAG` and is
  unchanged, as what follows it runs either way.
- **The run is `caught_run`'s**: from the statement after the `let`, to
  the last that mentions `x` or a name the run declares, at any depth,
  repeated until that adds none. A name JS gives to a Rust shadow that
  reuses one the run declares, `let shout` set again as `shout = ..`, is
  a mention, so the run takes the shadow too.
- **A payload a shadow sets again**, `let x = k * 3;` after `let Err(x)`,
  is `let x = $unwound(thrown);` in the `catch`, as lowering's
  `let { _0: x }` was.
- **A function the run declares that's called before it**, which JS
  hoists, keeps the form before the `try` (`$catchUnwind`), as moving it
  into the `catch` would put it out of reach of that call.
- **What the run does it does in Rust's order**: a `?`'s `return`, a
  `return`, a `continue` or a `break` leaves from the `catch` as it would
  from after it, and a guard the run makes is dropped where its scope
  ends, its `finally` in the `catch`; a guard made before the `let` holds
  the `try` and the rest in its own.

## Why

- **It's React's JS**: the `catch (x)` holds what reads `x`, as React's
  does, with no `let` before the `try` for it to set.
- **It's the Rust rustc asks for.** `if let Err(x)` of a closure that
  never returns is the irrefutable pattern rustc warns of; its
  suggestion, the `let` and the block's statements, now prints as the
  `if let` did.
- **It's exact.** The `try`'s body always throws, so the `catch` always
  runs, and what follows the `try` runs only after it: a statement moved
  into the `catch` runs once, where it ran. Nothing after the run reads
  what the run declares, so the `catch`'s block scope hides nothing.

## Alternatives

- **ADR 0706's: up to the last read of `x`, what that declares a `let`
  before the `try`.** Exact too, but not React's: `let stack;` and
  `stack = ..` where React declares `const stack` in the `catch`, and
  what reads it after the `try`.
- **The whole rest of the block in the `catch`.** Exact, but what reads
  neither the payload nor what it made, `render()` after `control = x`,
  would leave React's place after the `try`.
- **Keep `if let Err(x)`.** rustc warns of it; a port writes Rust's
  suggestion, the compiler adapting to it.

## Consequences

- The guide's `controlStack`, `let control = ..; control.and_then(..)`,
  is `catch (x) { const control = x; return control?.stack; }`.
- ADR 0706's mutation `unwound-let-declared-inside` goes with the `let`
  it tested.
- A `let Err(x) = catch_unwind(never)` of a named `fn never() -> !` keeps
  `$catchUnwind(never)`, as before: only a closure's body is known here
  to throw last.

## What holds it

- `test/lowering-unwinds.test.ts`, "what follows an irrefutable let Err of
  catch_unwind is its catch": React's `describeBuiltInComponentFrame`,
  its JS and its prefix and suffix, run in Node; "a port throws an Error
  by panic! and catches it as thrown" and "what's given the thrown
  downcast is given it as it is", whose returns are in the `catch`; "a
  catch_unwind that heads an if let chain is a try", whose `control =
  x;` has the closure's call after it.
- Corpus case `catch_unwind_irrefutable` against native Rust: React's
  shape on thread-locals, set once; the control a closure sets, what
  follows after the `try`; a run made longer by what it declares, a
  shadow after it; the payload's own name shadowed; a `?`; guards made
  before and in the run, a `return` between; a `continue` and a `break`
  in a loop; the run's value returned; a function hoisted; an `if let`
  of a function that may return.
- Mutations `caught-run-unclosed`, `unwound-let-hoisted-moved` and
  `unwound-let-shadow-const` (`prepare/catches.rs`).
