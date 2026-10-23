# 0709. A signed counter below a length stays below it, past its loop and through its `as`

Status: Accepted. Extends [0701](0701-a-signed-index-tested-not-negative-is-a-js-index.md)'s
ranges, and [0576](0576-what-a-bodys-integers-can-be.md)'s widening, as
[0632](0632-a-js-strings-length-is-below-u32-max.md) and
[0682](0682-a-js-strings-length-is-at-most-int32-max.md) have it.

Case ([0262](0262-when-rust-and-js-disagree.md)): A, the program can't
observe it. Each index left unchecked is one Rust's check passes, and each
cast left out one that keeps the number.

## Context

React's `describeNativeComponentFrame` (shared/ReactComponentStackFrame.js),
ported, counts its indices as `isize` from the start, as they go below
zero later:

```rust
let mut s: isize = 0;
while s < sample_lines.len() as isize && !sample_lines[s as usize].contains("DetermineComponentFrameRoot") {
    s += 1;
}
// .. and `c` of `control_lines` the same
if s == sample_lines.len() as isize || c == control_lines.len() as isize {
    s = sample_lines.len() as isize - 1;
    // ..
}
while s >= 1 && c >= 0 {
    if sample_lines[s as usize] != control_lines[c as usize] {
```

rust-js wrote `!$index(sampleLines, s >>> 0).includes(..)` in the first
loop, and `$index(sampleLines, s) !== $index(controlLines, c)` in the
last, where upstream reads `sampleLines[s]`. Each read is in bounds: `s`
starts at 0, grows only while it's below the length, and past the `if`
it's either the length less one or not the length.

ADR 0701's ranges lost each:

- **The first loop's counter was forgotten.** Widening took a growing
  counter as at most one below its type's most, so its `+ 1` can't wrap;
  then at most the length's most where it's bound by a length. Of an
  `isize` below `len() as isize`, both are `2^31 - 1`, and taken as one
  below it, `isize::MAX - 1`, its `s + 1` reached `isize::MAX` the next
  round, past what it was taken as, and it was forgotten: anything, so
  `s as usize` may change it, and `$index(.., s >>> 0)`.
- **`s != len() as isize` showed nothing.** ADR 0701's "`i != xs.len()`
  of an `i` at most the length puts it below it" saw through the `as` of
  the counter, not the one of the length.

## Decision

**A counter bound by a length is widened to the most that bound is, up
to its type's most** (`walk_loop`), not one below its type's most: the
length holds it, not a `+ 1` that can't wrap. A counter bound by no
length is one below its type's most, as before. Widening is a guess the
next round checks, so it stays sound either way: a counter that does
pass it is forgotten.

**An `as` keeps a slice's length where the length is in its target's
range** (`Walk::cast_of`, which `uncast` asks): `len() as isize` of items
that take up room, at most `isize`'s most (ADR 0701), is the length. So
`s != xs.len() as isize`, of an `s` at most the length, puts `s` below
it, as `s != xs.len()` does. Of items of no size, a length may be past
`isize`'s most, and its `as isize` is no length.

So both of React's loops read as upstream's:

```js
while (s < sampleLines.length && !sampleLines[s].includes('DetermineComponentFrameRoot')) {
  s++;
}
// ..
while (s >= 1 && c >= 0) {
  if (sampleLines[s] !== controlLines[c]) {
```

A counter that may still be the length keeps Rust's check where it's
read: `lines[s as usize]` after the loop, untested, is `$index(lines, s)`,
and panics where nothing matched, as Rust does.

## Why

- **It's upstream's JS**: `sampleLines[s]`, `s++`.
- **One rule for each**: a length bound is what holds a counter by a
  length, whatever its type; an `as` that keeps its value is seen
  through on either side of a test, its integer's and its length's.

## Alternatives

- **Write the port's counters as `usize`, and cast past the loop**, as
  `test/corpus/signed_index.rs` does: Rust bent for the compiler, which
  AGENTS.md rules out.
- **Take an `isize` counter's most as `isize::MAX - 1` and the length
  one below it**: a slice of sized items may hold `isize::MAX` of them
  as far as Rust says, and its length is its most, not one below.

## Consequences

- `test/corpus/signed_counter.rs` runs the port's loops natively and in
  JS: a common root, stacks matched from their ends, one line each, none,
  and a counter past its loop that is the length, which panics.

## What holds it

- `test/lowering-numbers.test.ts`, "a signed counter below a length stays
  below it past its loop", run in JS: both reads unchecked, and the one
  past the loop checked.
- Mutations `widened-length-counter-below-type-most`,
  `ne-length-cast-not-seen`, `cast-of-length-unknown`,
  `cast-of-length-past-isize`.
