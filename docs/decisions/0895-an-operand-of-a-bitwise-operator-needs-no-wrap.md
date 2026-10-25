# 0895. An `i32`'s `<<` and an operand of a bitwise operator need no wrap

Status: Accepted. Extends [0011](0011-numbers.md), which keeps each
integer's result in range with a wrap, `x | 0` or `x >>> 0`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference: each wrap left out is one JS's operator makes itself, of the
same bits, so every value computes as it did.

## Context

React's ReactFiberLane writes `1 << index`, `lanes & -lanes` and `(1 <<
(31 - clz32(lanes) + 1)) - 1`. Its port, of `i32` lanes, printed `(1 <<
i) | 0`, `lanes & (-lanes | 0)` and `(((1 << ((i + 1) >>> 0)) | 0) - 1) |
0`: ADR 0011 wraps each integer operation's result into its type's
range, and an operation's operand is another's wrapped result.

Rust-js's integers wrap as a release build's do (ADR 0011, the
semantics' first difference), so a wrap is needed only where JS's own
result could be out of the type's range.

## Decision

**A wrap is left out where JS's operator wraps anyway:**

- **An `i32`'s `<<` is JS's `<<`**, `1 << i`: JS's result is an int32,
  the `i32` Rust's release build computes, its amount masked to 31 as
  Rust's is. A `u32`'s keeps its `>>> 0`, and a narrower type's its
  wrap: JS's `<<` gives neither's range.
- **An operand of `&`, `|`, `^`, `~`, a shift and `Math.clz32` has no
  wrap of its own**, `lanes & -lanes`, `(a + b) & mask`, `~(a - b)`, `1
  << (i + 1)`, `Math.clz32(lanes)` of `(lanes as u32).leading_zeros()`:
  JS takes each of these operands modulo 2^32 itself, ToInt32 or
  ToUint32, and either of those of `x | 0` or `x >>> 0` is that of `x`.
  That holds of every number `x`, and the operand is a number, an
  integer type's, whose conversion reads nothing else.

**What's no such operand keeps its wrap**: `-lanes | 0`, whose
`i32::MIN` is itself, where JS's `-` gives 2^31; `((1 << (i + 1)) - 1) |
0`, whose `(1 << 31) - 1` is `i32::MAX`, where JS's `-` gives -2^31 - 1.
Upstream's `(1 << (i + 1)) - 1` is that number, out of an int32's range;
React's lanes never reach bit 31, which rust-js can't tell, so the port's
Rust and its JS keep `i32`'s answer.

Lowering decides it, where it makes each operator
(`std_types/number/operators.rs`'s `bits_operand`): the operand is the
expression it's given, its wrap there to see.

## Why

- **It's the JS a person writes**, upstream's `1 << index` and `lanes &
  -lanes`, and drops a wrap the reader must reason away.
- **It's exact**: ToInt32 and ToUint32 are each modulo 2^32, so applying
  one twice, or the other after one, is applying it once, for every
  number; a wrap's `x` is a number, so nothing it converts, a `valueOf`,
  could run apart from where it ran.

## Alternatives

- **Range analysis of each `-`**, ADR 0576's, to leave out a negation's
  wrap where its operand is never `i32::MIN`: it would be needed only
  where the `-` is no bitwise operator's operand, and none of React's is.
- **Preparation**, on the finished JS: it can't tell an integer's `x | 0`
  from a program's `x | 0` of a value of any shape, a `js::Unknown`'s,
  whose conversion may run a `valueOf` at another time.

## Consequences

- Six corpus programs, a snapshot and a comment test drop a wrap:
  `tagged_pointers.rs`'s `(value & ~STATE_MASK) >>> 0`,
  `char_from_code.rs`'s `((b & 0x1f) << 6) >>> 0`, `1 << by` and `a <<
  s` of `integer_families.rs` and the integer matrix, a generic `shl`'s
  `a << b`, and `Math.imul(a, 1 << 30)` of the numbers snapshot.

## What holds it

- `test/lowering-bits.test.ts`, "a shift and an operand of a bitwise
  operator are unwrapped", run in JS, with `i32::MIN`, shifts by 31, 32
  and 33, and the wraps kept; `test/corpus/lane_bits.rs`, against native
  Rust, at the same edges.
- Mutations `i32-shift-left-unwrapped-for-u32-too`,
  `bitwise-operand-of-any-or-unwrapped`, `negation-unwrapped`
  (`lower/std_types/number/operators.ts`), `clz32-argument-kept-wrapped`
  (`lower/std_types/number.ts`).
