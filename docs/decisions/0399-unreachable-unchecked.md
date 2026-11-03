# 0399. `hint::unreachable_unchecked()` is unreachable code's throw

Status: Accepted.

Case: A ([0262](0262-when-rust-and-js-disagree.md)): a program can't
observe it. Reaching `unreachable_unchecked()` is undefined behaviour, so
a program that's right never does, and what JS does there is JS's.

## Context

`unsafe { std::hint::unreachable_unchecked() }` tells the optimizer an arm
can't run: `match c { b'0'..=b'9' => .., _ => unreachable_unchecked() }`.
rust-js refused the call, and rustc's `for-loop-while/issue-69841.rs`
failed on it.

## Decision

**`hint::unreachable_unchecked()` is what MIR's unreachable code is,
`unreachable_code`'s one throw: `throw new Error("internal error: entered
unreachable code")`.**

```js
function digit(c) {
  if (c < 48 || c > 57) {
    throw new Error("internal error: entered unreachable code");
  }
  return (c - 48) & 255;
}
```

- **One form**: the hint and `unreachable!()`'s arm read the same; a
  throw, where nothing would, says where a wrong program went.

## Why

- **It's tested**: the corpus's `unreachable_hint.rs` runs with native
  Rust; the rustc test passes. A mutation leaves the throw out.

## Amendment: `assert_unchecked` and `unchecked_add`

Two more promises a program that's right keeps, as deranged's ranged
integers make them, were refused. Case N
([0262](0262-when-rust-and-js-disagree.md)): a program that keeps them
can't tell; one that breaks them is undefined behaviour, which Rust's
release build doesn't check either.

- **`hint::assert_unchecked(c)`** is nothing: its `c` is made as it's
  given, so `assert_unchecked(loud(x) <= 9)` still calls `loud`, and a
  condition of no effect, `MIN <= value && value <= MAX`, is gone.
- **`unchecked_add`, `_sub` and `_mul`** are what `a + b` is, `wrapping_*`'s
  form: in range, the same number.
- The `assert_unchecked` and `unchecked_and_saturating_neg` corpus cases
  run against native Rust.

## Amendment: a way to it alone is left out (rd643)

Ported React declares a type wider than any value its code makes:
encodeReply's `Promise<string | URLSearchParams | FormData>`, of an
untagged `ReplyBody` whose `UrlSearchParams` no code builds. Its `match`
then tested, and threw for, a variant that's never there: `else if (body
instanceof FormData) { .. } else { throw .. }`, where upstream tests
`typeof body === 'string'` alone. The user chose the form a Rust
programmer writes for "never this": `ReplyBody::UrlSearchParams(_) =>
unsafe { std::hint::unreachable_unchecked() }`.

**`Mir::new` makes each call of `unreachable_unchecked()` MIR's
unreachable code (`recognition::is_unreachable_hint`), and a block that
only goes to it, past `goto`s and a `match`'s false edges, is unreachable
(`queries::is_unreachable`).** Every body rust-js lowers is made by
`Mir::new`, so no call of it is left to recognize: `Std::Unreachable`,
its classification, is gone with its mutation, which survived. A
switch leaves out its ways to such a block, as it leaves out MIR's own
(`structure.rs`), so a `match`'s arm of only the hint goes with its test,
and the last arm it tests is what's left:

```js
function size(body) {
  if (body.TAG === "Text") {
    return $byteLen(body._0);
  }
  return body._0.length * 2;
}
```

- **Case A** ([0262](0262-when-rust-and-js-disagree.md)) still: reaching
  the hint is undefined behaviour, so no program that's right sees its
  test go. rustc's optimizer drops the same tests.
- **`unreachable!()` keeps its throw.** It's a panic, which a program may
  catch, not a promise; only the hint is left out. The "one form" above
  no longer holds: where the hint isn't all a way does, or a function's
  body is only the hint, it still throws, as MIR's unreachable code does.
- **An `if` too**: `if n % 2 == 1 { unsafe { unreachable_unchecked() } }`
  is nothing. What its test makes is still made: a switch that goes one
  way alone keeps its subject's effects as a statement, `loud(n);` of
  `if loud(n) { .. }`, where it had dropped it (this ADR's corpus case).
- **A loop is reached**: the walk stops at any other terminator, a
  loop's `falseUnwind` among them, and at a block it's seen, so a `goto`
  of itself, as an optimized std body's `loop {}` is, isn't followed
  forever. That guard has no test: no body rust-js lowers from its own
  crate has one (borrowck's MIR keeps `falseUnwind`, and a `const fn`'s
  evaluated MIR counts each turn, `ConstEvalCounter`).
- **Tested** by the corpus's `unreachable_hint.rs`, against native Rust,
  and the mutations `unreachable-hint-a-call`,
  `unreachable-past-false-edge-unseen` and
  `untested-switch-subject-dropped`; `unreachable-hint-falls-through`
  now leaves out MIR's unreachable code's throw, which a function whose
  body is only the hint keeps.
