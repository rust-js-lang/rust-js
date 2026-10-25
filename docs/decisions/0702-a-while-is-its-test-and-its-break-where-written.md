# 0702. A `while` is its test, and its `break` is where it was written

Status: Accepted. Extends tidy's `while_tests` and
[0683](0683-an-if-is-as-written-and-an-else-if-stays-a-chain.md)'s `if`
as written, which left a loop's test in its body.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. How the JS reads;
nothing that runs changes.

## Context

React's `describeNativeComponentFrame` (shared/ReactComponentStackFrame.js)
has two loops its port writes as Rust does:

```rust
while s < sample_lines.len() && !sample_lines[s].contains("DetermineComponentFrameRoot") {
    s += 1;
}
while s >= 1 && c >= 0 {
    if sample_lines[s as usize] != control_lines[c as usize] {
        // ..
        break;
    }
    s -= 1;
    c -= 1;
}
```

rust-js wrote neither as a `while` of its test:

```js
while (true) {
  if (s >= sampleLines.length || sampleLines[s].includes("Root")) {
    break;
  }
  s++;
}
while (true) {
  if (s >= 1 && c >= 0) {
    if (sampleLines[s] !== controlLines[c]) {
      // ..
    } else {
      s--;
      c--;
      continue;
    }
  }
  break;
}
```

A `while` is `loop { if test { body } else { break } }` to rustc. Where
its test calls, a `Vec`'s index here, ADR 0683's `written elses` keeps
that `if` as written, its `else` the `break`; `while_tests` took a loop's
test only from `if (t) { break; } rest`. In the port, inside its guard's
`try`s, the second also took a labeled block, `bb163: { .. break bb163;
}`, round the `while (true)`.

## Decision

**`while (true) { if (t) { rest } else { break; } }` is `while (t) {
rest }`**, and **`while (true) { if (t) { rest } break; }` is `while (t) {
rest; break; }`**, of a loop the program wrote as a `while`
(`while_tests`). The `break`, and the `continue` below, are of no label:
in the loop's body and no loop in it, that's the loop's own, so no label
is compared. Of that `break` after the `if`, where `rest` ends `if (u) {
a } else { b; continue; }`, the `break` is `a`'s and `b` the turn's rest
after it, `if (u) { a; break; } b`, and of `if (u) { b; continue; } else
{ a }`, `if (u) { b } else { a; break; }`, as written (`broken_after`):

```js
while (s < sampleLines.length && !sampleLines[s].includes("Root")) {
  s++;
}
while (s >= 1 && c >= 0) {
  if (sampleLines[s] !== controlLines[c]) {
    // ..
    break;
  }
  s--;
  c--;
}
```

A `loop` the program wrote stays `while (true)`, its `break` where it's
written: upstream's `do { .. } while (s >= 1 && c >= 0)` is the port's
`loop { ..; if !(s >= 1 && c >= 0) { break; } }`, which JS's tree has no
`do … while` for yet.

## Why

- **The loop the program wrote**: its test is the `while`'s, as Rust and
  upstream have it, not an `if` and a `break` the reader puts back
  together.
- **One rule for a `while`'s test**, wherever its `if` came from:
  `while_tests` takes it of either form.

## Alternatives

- **Undo `written elses` in a loop's head**: the `if` the program wrote
  with an `else` would lose it elsewhere too. Rejected.
- **A `for (; t; s--, c--)`**: upstream's, but the port writes `s -= 1;
  c -= 1;` as statements after the `if`, which a `for`'s update would
  move. Left for when a `for` the program can't write in Rust is asked for.

## What holds it

- `test/lowering-written-loops.test.ts`, "a while is its test, its break
  where it was written", run in JS.
- `test/corpus/signed_index.rs`, React's loops, run in both.
- Mutations `while-of-else-break-unread`, `while-of-tail-break-unread`,
  `broken-after-continued-branch`, `broken-after-continued-then`,
  `broken-after-then-unbroken`, `broken-after-else-unbroken`.

## Amendment: an exit that goes on with the loop stays in it

React's `findFirstSuspended` walks up while a fiber has no sibling:

```rust
while node.sibling.borrow().is_none() {
    let return_fiber = node.r#return.borrow().clone();
    match return_fiber {
        Some(return_fiber) if !Rc::ptr_eq(&return_fiber, row) => node = return_fiber,
        _ => return None,
    }
}
```

MIR tests it as `while (true) { if (node.sibling == null) { .. continue;
.. break; } rest }`, and tidy's `while_tests` wrote `while (!t) { rest }
exit` of an exit that leaves: the exit after the loop, its `continue`,
which went on with the inner loop, then the outer one's. The JS climbed
back to a fiber's child it had left, and never ended: a wrong answer.

**An exit that goes on with the loop, a `continue` of it, stays in it**
(`js::continues`): the loop is `while (true) { if (t) { .. } .. }`, as
MIR has it. Case N: what runs is Rust's.

- `test/corpus/while_exit_continues.rs`, beside native Rust; mutation
  `while-exit-continue-moved`. Its loops, `bb2: while (true)` around
  `while (true)`, count one more of `docs/readability.txt`'s labeled
  blocks and `while (true)`s, blessed: the JS that runs as Rust does. No
  other corpus case's or snapshot's JS changed.

## Amendment: no `break` after what leaves itself (#290)

`while_tests` makes `while (t) { rest }` of a loop its function ends
with, and appends the loop's `break` to `rest`, leaving it for
`unreachable` to drop. When `rest` already ends with what leaves, such
as a panic's `throw`, the `break` stayed: the `chains` stage runs no
`unreachable` after it, so tidy's contract "nothing after a jump" broke
and the compiler panicked (fuzz seed 61706). Now `broken_after` appends
no `break` after a `break`, `continue`, `return` or `throw`.
`while_test_ends_in_throw` compiles such a loop and runs it against
native Rust, and `broken-after-a-throw` is caught.
