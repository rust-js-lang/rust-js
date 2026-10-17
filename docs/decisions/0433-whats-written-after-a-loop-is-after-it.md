# 0433. What's written after a loop is after it

Status: Accepted. Extends [0370](0370-loop-exits-first.md) and
[0432](0432-a-return-is-written-where-its-value-is-made.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)): where a statement
that runs once a loop is left is written changes nothing a program can
observe, only how its JS reads.

## Context

MIR has no loop exit of its own: a `break` jumps to the block after the
loop, and when that block is reached from nowhere else, rust-js wrote it
where the `break` was, inside the loop:

```js
while (p.at % 3 !== 2) {
  const args = [];
  while (true) {
    // ..
    if (p.at % 2 === 0) {
      e = { TAG: "Call", _0: e, _1: args };
      continue bb1;
    }
  }
}
```

A lexer's `out.push(..); continue;` after its inner loop, `main`'s every
statement after its first `while let`, and a function's `Ok(tokens)` after
its loop read so. `loop_exits` moved one case out, a function ending in a
loop left by one `return` MIR made, but late, after the passes that make a
loop's early `break`s had run, so `while let Some(c) = peek { if .. {} else
{ break } }` stayed `if (c != null) { if (..) { ..; continue; } } break;`.

## Decision

**A `while (true)` that nothing `break`s, whose one way out does what the
program wrote after the loop, `break`s there, and what it does is written
after the loop** (`exits_after`).

```js
while (p.at % 3 !== 2) {
  const args = [];
  while (true) {
    // ..
    if (p.at % 2 === 0) {
      break;
    }
  }
  e = { TAG: "Call", _0: e, _1: args };
}
```

- **Written after it, by where Rust has it:** the way out's last statement
  leaves the loop, a `return`, a `throw` or a jump past it. What it does
  from there back is each written after the loop, or made by MIR, a
  `return` MIR made where its value is written. At least one of them is
  written there. Back past a jump to a label inside the loop it doesn't
  go. A way out inside what moves moves with it, and of two ways out
  neither moves.
- **Not out of scope:** nothing moved reads a name the loop declares. Its
  own names are its Rust block's, named apart from those beside the loop.
- **One way:** `loop_exits` is gone, and the `valued` hint only it read.
  `exits_after` runs before a loop's early `break`s are made, so each of
  its cases reads better.
- **A loop's early `break`s, of any jump** (`early_breaks`): `if (c) { a;
  return x; } break;` is `if (!c) { break; } a; return x;`. Once the tests
  are read in place, a nesting of them too, `if (a) { x; if (b) { y;
  continue; } } break;`, `if (!a) { break; } x; if (!b) { break; } y`,
  then the loop's head, `while (a)`. Before that, `if (b) { if (c) { .. }
  }` of nothing else is one test, `b && c`.
- **Written order** (`written_order_steps`): `if (t) { a } rest`, `rest`
  going round at the body's end, is turned where `rest` is written first
  and `a` is more than a lone exit, a guard's: `rest` gets the `continue`.
  Where a statement is written (`first_written`) looks into a `try`, and an
  `if` is placed by its branch, not its test, a `match`'s scrutinee
  written before every arm.

## Why

- **It's the JS a person writes:** a `while let`'s exit first, the code
  after a loop after it. The interpreter's lexer reads line for line as
  its Rust. `iter_mut_kept`'s, `stock_orders`' and `slice_split_off`'s
  `main` is no longer nested in a loop's `else`. A `Display` `match`'s
  first arm comes first, as written, where it was the last.
- **It's right by construction:** a way out that leaves the loop runs what
  it holds, then leaves; a `break` there, then the same statements after
  the loop, is the same. The `loop_exit_moves` case has a value with a
  destructor dropped on the way out, a name declared before the loop and
  set in it, a shadowed name, a loop after the loop, a labeled loop, a
  written `return` and a valued `break`, each against native Rust. A
  mutation for each step is caught.

## Consequences

- A function's `None` after its loop is `return;` after it, as written,
  where MIR's `return` was inside the loop.
- Code that moves out of a loop beside many of a function's own temporaries
  takes the next free name, `value$16`, where inside the loop it reused
  `value`.
