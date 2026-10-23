# 0631. A step of one is `++`

Status: Accepted. Extends [0011](0011-numbers.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): A: how a step is
written, not what it computes.

## Context

React's `normalizeConsoleFormat` steps its counters with `j++`, as JS
writes a counter's step. Its port wrote `j += 1;`: preparation makes
`x = x + 1` the compound `x += 1` (`prepare.rs`'s `compound`), and the
printer wrote every compound with its operator, so a counter read as no
JS developer writes it. A `for` loop over a range already wrote its
update `i++`.

## Decision

**A statement `x += 1` is `x++`, and `x -= 1` is `x--`**, of a variable,
a field or an item: `j++`, `counter.hits++`, `counts[i]--`. Any other
step is the compound as before, `j += 2`, `s += "1"`.

The printer decides it, where a compound statement is written
(`to_oxc/statements.rs`), and nowhere else: preparation still makes each
`x = x op v` the one compound, and the printer writes a compound whose
step is the number literal `1` as JS's update expression. It's only a
statement: no expression holds a compound, so `x++`'s value, the old
one, is never read.

The runtime's own helpers step the same way, `channel.senders++`,
`child++`, so what rust-js writes has one form of a step.

## Why

- **One form of output for each construct**: a step of one was `i++` in a
  `for` and `i += 1` in a `while`; it's `++` in both now.
- **Only a number's**: in Rust's types a `+= 1` whose right side is the
  number literal `1` is a number's. A string's `+=` takes text, a
  BigInt's step is `1n`, another literal kind, and an `i64`'s or a
  `u64`'s is written with its wrap, `BigInt.asUintN(64, n + 1n)`, never
  a compound. JS's `x++` of a number is `x += 1` of it, `-0`, `NaN` and
  `Infinity` alike, so nothing a program can observe changes.
- **A wrap stays a wrap**: `x = (x + 1) >>> 0` of a `u32` that can wrap is
  no `+= 1`, and stays as it is.

## Alternatives

- **Decided in preparation**, a statement kind of its own made where the
  compound is: every pass that walks statements would learn a new kind,
  for a question only how it's printed asks.
- **`++` inside an expression too**: no compound is one, and `x++`'s value
  isn't `x += 1`'s.

## Consequences

- Nine corpus programs, four snapshots and three guide examples write
  `n++`, `n--`, `v.value++`. `docs/readability.txt` doesn't move: no shape
  it counts is a compound.

## What holds it

- `test/lowering-numbers.test.ts`, "a statement stepping by one is ++ or
  --", run in JS.
- Mutations `step-of-any-number-incremented`, `step-down-incremented`.
