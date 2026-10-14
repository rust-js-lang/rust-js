# 0372. A check whose panic drops is written where it's made

Status: Accepted. Amends [0364](0364-mir-lowering.md),
[0098](0098-destructors.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what a panic drops as
it unwinds is printed by the program, so it's Rust's, paid for in an `if`.

## Context

MIR checks a division by zero, and an index, with an `Assert` whose
unwinding drops what's in scope. Lowering left the check to the helper
that runs next, `$div(n, by)` or `$index(xs, i)`, which throws the same
panic, but not inside the `try` that drops: a value made before it, or one
a variable owns, wasn't dropped as the panic unwound.

```rust
let checked = pair(Noisy("made"), n / by); // by == 0: no `drop made`
```

Whether an `Assert` is written was also asked twice, by the count of what
each local is read and by the terminator, each with its own list; changing
one left the other counting reads of a check no longer made.

## Decision

- **One answer to how an `Assert` is written**, `assert_check`: left out
  where it passes, THIR's facts know so or its index and length are
  constants, or where the helper that runs next checks it; made at the
  element's read, `$index`; or an `if` that throws. Both readers ask it.
- **A helper checks only what drops nothing.** Where the check's unwinding
  drops something (`drops::cleans_up`), it's the `if`, whose panic drops
  it first:

  ```js
  if (by === 0) {
    noisyDrop_drop(value);
    throw new Error("attempt to divide by zero");
  }
  ```
- **The destructors' facts keep what's read**: what's refused of a body,
  and its momentary borrows (ADR 0328). What moves an owner, and which
  temporaries hold one, MIR's drop elaboration has said since ADR 0364;
  the THIR walk's own record of them, and `effects.rs`, which only it
  asked, are gone.

## Consequences

- `drop_operand_leaves` and `drop_index_panic` run a panic's drops against
  native Rust; `slice_uninit`'s read is checked where its vector's items
  are dropped.
- Not yet: the flag of a temporary is named for its MIR local,
  `_14$live`, and a call it's moved into keeps a `try` whose flag is
  already `false`.
