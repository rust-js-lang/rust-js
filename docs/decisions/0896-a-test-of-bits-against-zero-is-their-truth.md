# 0896. A test of bits against `0` is their truth

Status: Accepted. Extends tidy's `written` stage, beside `turned`'s `!(b
>= 48)`, `b < 48`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference: a bitwise operator's value is a number of 32 bits, never
`NaN`, so it's truthy exactly where it isn't `0`; and a test's truth is
all an `if`, a `while` or a conditional reads of it.

## Context

React's ReactFiberLane tests a lane set's bits by their truth: `if
(lanes & SyncHydrationLane)`, `if (updateLane & IdleLane)`. Rust's `if`
takes a `bool`, so its port writes `if lane & SYNC_HYDRATION_LANE != 0`,
which printed `if ((lane & SyncHydrationLane) !== 0)`, fourteen times in
`getLabelForLane` alone.

## Decision

**A test `b !== 0` of a bitwise operator's value `b` is `b`, and `b ===
0` is `!b`**: `if (lane & 8)`, `if (!(lane & 4))`, `while (!(lanes & 1)
&& ..)`, `on && flags & 0x80 ? 1 : 2`. A `u32`'s `(a & b) >>> 0` is `0`
where `a & b` is, so its test is `a & b`.

- **Only of a test**: an `if`'s, a `while`'s and a conditional's, and the
  parts of an `&&`, `||` or `!` of one. A value stays a comparison,
  `return (a & b) !== 0`, a `bool`.
- **Only of bits**: `&`, `|`, `^`, `~` and the shifts, not an `i32`'s
  wrap `(n % 2) | 0`, which is arithmetic's, nor a variable, `n !== 0`:
  an integer's test of a count reads as the program wrote it, and a
  variable may be an `f64`'s, `NaN`.
- **Only against the literal `0`**: a named constant reads by its name,
  upstream's `(lanes & mask) !== NoLanes` stays so.

Tidy's `written` stage decides it (`logicals.rs`'s `bit_tests`), after
`turned`: the test's shape is final there, and the rule reads only JS's,
which a number of 32 bits a bitwise operator's value always is.

## Why

- **It's how JS tests bits**, upstream's form, and React's port reads as
  React does.
- **It's exact**: a number's truth is its being neither `0` nor `NaN`,
  and no bitwise operator gives `NaN`. A BigInt's test is against `0n`,
  never the number `0`, so no BigInt is tested.

## Alternatives

- **Every integer's `x != 0` as `x`**: `if (count)` of `if count != 0`
  reads less as written, and tidy can't tell an integer variable from a
  float's by its JS. Left out.
- **In lowering, where the comparison is made**: lowering doesn't know
  whether its value is a test's or a `bool` kept; the `!= 0` of `if lane
  & X != 0` is one MIR statement, its `switchInt` another.

## Consequences

- `user_fmt_traits.rs`'s `if (flags[0] & bit)`, `lane_bits.rs`'s
  `if (lanes & 2 && ..)` and the guide's `if (lane & SYNC_LANE)` read so;
  and `lowering-comments.test.ts`'s ReactFiberLane test is upstream's
  `if (// .. \n (lane & entangledLanes) | // .. \n (entanglements[index] &
  entangledLanes))`, its comments where they were, without `!== 0`.

## What holds it

- `test/lowering-bits.test.ts`, "a test of bits against 0 is their
  truth", run in JS, with a value, a variable and an arithmetic wrap left
  as they are; `test/corpus/lane_bits.rs` against native Rust.
- Mutations `bit-test-compared`, `bit-test-of-zero-kept-truthy`,
  `bit-test-of-a-value-too`, `bit-test-of-a-wrap` (`tidy/logicals.ts`).
