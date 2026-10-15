# 0372. A check whose panic drops is written where it's made

Status: Accepted. Amends [0364](0364-mir-lowering.md),
[0098](0098-destructors.md). Amended: what its constants say of a check is
`constant_truth`'s alone (below).

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
  where it passes, THIR's facts know so or its constants do, or where the
  helper that runs next checks it; made at the element's read, `$index`;
  or an `if` that throws. Both readers ask it.
- **What its constants say is `constant_truth`'s**, of the `Assert`'s own
  condition in MIR: an index below a length, `2 < 3`, a divisor against
  0, `2 == 0`, each of a local assigned a constant once, `!` of one, or
  `&` of two, as a signed division's overflow, `2 == -1 & n == MIN`, false
  of one. (Amended: a constant index was asked of again where it's read,
  `within`, and the `if`'s test of again as JS, `truth()`, which alone saw
  a constant divisor; neither is now, as neither was reached for an index.)
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
- **A flag known where it's tested is no test** (`mir/tidy/flags.rs`):
  MIR's cleanup is shared by every call that may panic, so its drop tests
  the flag even just after it's set `false`, as the value was moved into
  the call. Tidy follows each flag through straight-line code: a test of a
  known one is the branch it takes, a `try` whose `catch` then only throws
  on is none, a panic's own `try` is its drops then its `throw`, and a
  flag nothing reads goes. Where a panic may come before the flag is set,
  in a `try`, or a loop's earlier time round set it, it isn't known. A
  temporary's flag is named for the variable its drop is of, and a part's
  by its field, `pair$a$live`. Thirty-two corpus programs lost about 3,300
  lines; `drop_update_panics` and `drop_flag_loops` hold the two places a
  flag isn't known.
