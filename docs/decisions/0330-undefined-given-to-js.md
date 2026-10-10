# 0330. An `undefined` given to JS is an argument

Status: Accepted. Amends [0313](0313-console-forms.md); extends
[0021](0021-js-interop.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

ADR 0313 left a trailing `undefined` out of every call, `f(x)` for `f(x,
None)`, as a person leaves an optional argument out, and filed what it
changes as case B: only `arguments.length` tells. A review found more
tells: a binding of `Math.max` called `math_max(1.0, None)` was
`Math.max(1)`, which is `1`, where `Math.max(1, undefined)` is `NaN`.
`console.log`, `push` and every variadic JS function count their
arguments too.

## Decision

**A trailing `undefined` is left out only of a call of rust-js's own: the
crate's function, a closure of the crate's, a runtime helper. One given to
JS, a binding's or a `dyn Fn`'s, a generic `F`'s or a function pointer's,
any of which may be JS's, is kept.**

```rust
math_max(1.0, None);   // Math.max(1, undefined)
f(1.0, None);          // f(1, undefined), f: &dyn Fn(f64, Option<f64>)
own(1.0, None);        // own(1)
```

- **The contract is in the JS it makes**: what's given to JS is
  `GivenUndefined`, which prints `undefined` and which no pass leaves out;
  rust-js's own calls give `Undefined`, which prepare leaves out where it's
  last, as before. rust-js's code never asks how many arguments it was
  given.
- **What changes**: a JS function given `undefined` where one was left out,
  react.dev's `listen(listener, undefined)` say, whose `listen` takes it as
  missing, as it does an argument left out.

## Why

- **It's the same program**: a JS function gets the arguments Rust gave it.
- **It's tested**: `test/runtime-package.test.ts` calls a binding of
  `Math.max`, a `dyn Fn` given `Math.max`, and the crate's own function and
  closure, each with `None`, in node: `NaN`, `NaN`, and the omitted ones
  `own(1)` and `add(1)`. Mutations leave it out of a binding's call and a
  `dyn Fn`'s, and keep it in a closure's of the crate's.
