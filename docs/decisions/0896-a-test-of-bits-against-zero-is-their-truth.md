# 0896. A test of bits against `0` is their truth

Status: Accepted. Extends tidy's `written` stage, beside `turned`'s `!(b
>= 48)`, `b < 48`. Amended (2026-10-26): any integer's test against `0`
is its truth too, `if (lanes)`, not only a bitwise operator's.

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

## Amendment: any integer's test against `0`

React's ReactFiberLane tests lane sets by their truth where they aren't
bits of an operator too: `if (unblockedLanesWithMatchingPriority)`,
`while (lanes)`. Its port's `if unblocked_lanes_with_matching_priority
!= 0` and `while lanes != 0` printed `!== 0`, which this decision left
to a variable, as "an integer's test of a count reads as the program
wrote it". Decided otherwise (2026-10-26, the user's): **any integer's
test against the literal `0` is its truth**, of the Rust types JS's
numbers hold, `i8` to `i32`, `u8` to `u32` and `usize`, and of a `u64`'s
BigInt the same where it's tested against a number `0`, which it isn't.

- **An integer's value, as tidy reads it** (`logicals.rs`'s
  `integer_value`): a variable each of whose locals is an integer (the
  `numbers` `turned` reads, `hints.rs`'s `typed_names`), a whole literal,
  a wrap, `(n + m) | 0` or `(a + b) >>> 0`, a bitwise operator's, and a
  sum, difference or product of those, or a remainder of one by a literal
  not `0`, `n % 2`: `if (!(n % 2))`, `if ((n + m) | 0)`. A wrap's test
  keeps its wrap: what it wraps may be `2 ** 32`, truthy where its wrap is
  `0`.
- **Not a float's**: an `f64` variable isn't in `numbers`, so `if (x !==
  0)` stays, `NaN` being unequal to `0` and falsy.
- **Not a name a closure in the body declares too**: a conditional's walk
  goes into closures, whose `n` may be a float where the function's is an
  integer. A closure's body is tidied as its own, with its own `numbers`,
  so no case found reaches it; the guard keeps it so where one would.
- **Not a `match`'s pattern's**, `match n { 0 => .., _ => .. }`'s `if (n
  === 0)` nor `0 | 6 =>`'s `day === 0 || day === 6`: a pattern compares
  what's matched with its constants, as a `switch`'s `case 0:` does, not
  its truth (`Written::patterns`, each arm's pattern's span).
- **After `switches`**: the pass runs once a `match` of constants is a
  `switch`, whose chain's `day === 0` it reads its cases by.
- **Not a call's or a property's**, `v.len() != 0`'s `v.length !== 0` or
  `!is_empty()`'s `updaters.size !== 0`: tidy can't tell a JS getter's
  value from Rust's integer by its JS. They read as before.

Case N, as the decision's own: an integer is never `NaN`, so it's truthy
exactly where it isn't `0` (and `-0` is `0` both ways).

Consequences: upstream React isn't one way. ReactFiberLane has both
`if (unblockedLanesWithMatchingPriority)` and `if (pendingSyncLanes !==
0)`; the first now reads as upstream, the second, which read as it,
doesn't. So do shared's `indent === 0`, `propsLength === 0` and
formatProdErrorMessage's `i === 0 ? ..`, now `!indent`, `!propsLength`,
`!i ? ..`, and the guide's `!n ? "zero" : "positive"`. A test of a
`while`, `while (lanes)`, reads as upstream's everywhere it's written so.
rust-js's own checks read so too: `if (!value$3) throw new Error("attempt
to calculate the remainder with a divisor of zero")`.

## What holds it

- `test/lowering-bits.test.ts`, "a test of bits against 0 is their
  truth", run in JS, with a value left as it is, an integer variable's and
  a remainder's test their truth, and a float's `x !== 0` kept, run with
  `NaN`; `test/corpus/lane_bits.rs` and `test/corpus/integer_truth.rs`
  against native Rust.
- Mutations `bit-test-compared`, `bit-test-of-zero-kept-truthy`,
  `bit-test-of-a-value-too`, `integer-test-compared`,
  `integer-test-of-a-float` (`tidy/logicals.ts`). `bit-test-of-a-wrap`,
  which took a wrap for bits, is gone: a wrap is an integer's value now.
