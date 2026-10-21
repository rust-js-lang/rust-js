# 0532. A `Debug` builder held in a variable

Status: Accepted. Extends [0136](0136-common-std-methods.md) and
[0137](0137-pretty-debug.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's written is
Rust's, plain or pretty, whichever steps a run takes.

## Context

A `Debug` builder stepped in a chain, `f.debug_struct("Pair").field(..)
.finish()`, is written as one string, each part known where it's made. One
held in a variable, stepped in a branch or a loop, is the other way Rust's
`Debug` impls are written, itertools' `ExactlyOneError` among them:

```rust
let mut dbg = f.debug_struct("Pair");
if let Some(first) = self.first {
    dbg.field("first", &first);
}
dbg.finish()
```

It was refused, "a `Debug` builder kept in a variable", since which steps
are taken isn't known until it runs.

## Decision

**A builder held in a variable is the array of its parts, each step's
pushed as it's taken, and `finish` writes what it was given, as a chain's
`finish` writes its parts.**

```js
const parts = [];
if (pair.first != null) {
  parts.push(`first: ${pair.first}`);
}
return parts.length === 0
  ? "Pair"
  : options?.alternate
    ? $pretty("Pair {", parts, "}")
    : `Pair { ${parts.join(", ")} }`;
```

- **One way to write a step and a finish**: `builder_part` writes a step's
  part, one or, of `entries`, many, spread where it's pushed; and
  `builder_assembled` writes the finish of parts, a chain's in place or a
  held one's array. A chain is the builder whose MIR local isn't a named
  variable's.
- A struct's or a tuple's given no field is its name alone, `Pair`, as
  Rust's, which only a held one asks at run time; `finish_non_exhaustive`
  of none is `Pair { .. }`, plain and pretty, as Rust's, which `$pretty`
  now writes so.
- What borrowck alone reads of a writer's locals, a `let`'s `FakeRead`,
  isn't a use of its `Formatter`.

## Why

`debug_builder_held` steps a held `DebugStruct` in an `if let` and a
`match`, a `DebugList` and a `DebugTuple` in a loop, a non-exhaustive
struct given a field or none, and a `DebugSet`'s `entries` and `entry`,
each plain and pretty, against native Rust. Every other snapshot is
unchanged.

## Consequences

- itertools' `ExactlyOneError`'s `Debug` compiles.
- Mutations take a held builder for a chain, add no part, add `entries`
  as one part, write a struct given no field as `Row()`, refuse its `let`,
  and write an empty non-exhaustive struct `Open {..}`.
