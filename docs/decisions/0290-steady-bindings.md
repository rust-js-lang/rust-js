# 0290. A pattern names a variable set again while nothing sets it

Status: Accepted. Extends the stable places of `if let` and `match`
subjects: a variable set again can be one.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

What a pattern binds of a place names the place where nothing can change
it while the binding is read: `if let Some(e) = error && e.message == ..`
is `if (error != null && error.message === ..)`. Only an immutable
variable counted. react.dev's Preview tests the error Sandpack gives it and
then clears it:

```ts
let {error: rawError} = sandpack;
if (rawError && rawError.message === '...') {
  rawError = null;
}
```

Its Rust, `if let Some(error) = rawError && error.message == ".." {
rawError = None; }`, gave `if (rawError != null) { const error = rawError;
if (error.message === "..") { rawError = undefined; } }`: a statement and
a name where the original has one condition.

## Decision

**What an `if let` or a `match` binds of a variable set again names it in
place where nothing may set it after the test and before what's bound is
read.**

```rust
if let Some(e) = raw && e.message == "noisy" { raw = None; }
// if (raw != null && raw.message === "noisy") { raw = undefined; }

if let Some(e) = raw { raw = None; use_it(e); }
// if (raw != null) { const e = raw; raw = undefined; use_it(e); }
```

What sets a variable: an assignment to it or into it, a `&mut` of it or
of a part, and a `ref mut` binding of it. A setting counts unless:

- it comes before the test, in the body's order;
- or every read of what's bound comes before it, and no loop the test
  isn't in holds both.

A read in a closure, and a `Cell` lent of what's bound, read the variable
when they're called, so any setting after the test counts.

A `let` still binds its own: `const [a, b] = pair` is already how JS
destructures a variable.

## Why

- **It's the JavaScript a person writes**: one condition, the variable read
  where it's tested.
- **It's the same program**: what's bound is read only while the variable
  still holds what was tested.

## Consequences

- A `match` arm's bindings name a parameter set later in place too: `long
  ${shape._0}`, not a `const n`.
