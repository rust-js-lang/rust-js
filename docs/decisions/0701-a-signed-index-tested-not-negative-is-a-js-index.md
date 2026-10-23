# 0701. A signed index tested not negative is a JS index as it is

Status: Accepted. Extends [0576](0576-what-a-bodys-integers-can-be.md)'s
ranges, past an `as`, with a slice's length of items that take up room at
most `isize`'s most; amends [0682](0682-a-js-strings-length-is-at-most-int32-max.md)'s
"an array's may be `2^32 - 1`" and
[0672](0672-a-property-a-test-reads-is-read-once.md)'s `returning_chain`,
whose bound reads nothing after reads are gone, as `and_tests`' are.

Case ([0262](0262-when-rust-and-js-disagree.md)): 1, the program can't
observe it. A cast left out is one that keeps the number, an index left
unchecked one Rust's check passes, a wrap left out one that wraps
nothing; a slice longer than `isize::MAX` items of some size is one no
Rust allocation holds.

## Context

React's `describeNativeComponentFrame` (shared/ReactComponentStackFrame.js)
matches a sample stack against a control one from their ends, its indices
going below zero where it runs out:

```js
s = sampleLines.length - 1;
c = controlLines.length - 1;
while (s >= 1 && c >= 0 && sampleLines[s] !== controlLines[c]) {
  c--;
}
```

Its port's indices are `isize`, as they go below zero, each read as
`sample_lines[s as usize]` under `s >= 1`, and rust-js wrote

```js
s = s | 0;
if (s >>> 0 === sampleLines.length || c >>> 0 === controlLines.length) {
  s = ((sampleLines.length | 0) - 1) | 0;
  while (s >= 1 && c >= 0 && $index(sampleLines, s >>> 0) !== $index(controlLines, c >>> 0)) {
```

Each is right where nothing is known (ADR 0011): `as isize` of a `usize`
past `2^31 - 1` is negative, `as usize` of a negative `isize` is past
`2^31`, and `$index` panics past the end. Here each is known, but the
ranges stopped at an `as`, which they took as anything of its type.

## Decision

**An integer `as` another whose range it's already in is the same
number** (`ranges.rs`, `Ranges::kept_casts`): its range is its operand's,
bound by a length too, and its JS is its operand's, no `| 0` or `>>> 0`.
`s as usize` under `s >= 0` is `s`, so `sample_lines[s as usize]` under
`s >= 1` and `s` below the length is `sampleLines[s]`. Where nothing shows
the sign, `items[i as usize]` keeps `$index(items, i >>> 0)`, and panics
of `-1` as Rust does. One set of the casts that keep their value,
`BodyFacts::kept_casts`, holds these and ADR 0302's whole floats, and
`kept_cast_at` answers for both.

**A slice's length, of items that take up room, is at most `isize`'s
most**, `2^31 - 1` (`SIZED_SLICE_LENGTH_MOST`): Rust allocates no more
than `isize::MAX` bytes, so `len() as isize` is the length, and `len() as
isize - 1` is `length - 1`. Of items of no size, or of a size not known,
a generic `T`'s, it's `u32`'s most still.

So that a counter below a length stays below it past its loop, three
more of what's known:

- **A range joined with one bound by no length is bound by its most**,
  `⌊(0·L + hi) / 1⌋`: `let mut s = 0` and an `s` below `xs.len()` are
  both at most the length.
- **A counter widened round a loop keeps a bound it still has** (ADR
  0632's widening), its most that bound's where the length is its most.
- **`i != xs.len()` of an `i` at most the length puts it below it**, and
  a test sees through an `as` that keeps its value.

MIR writes the operator an assignment is of, `s = n - 1`'s, where the
assignment is, so an exact one is known by that span too.

What the index's read leaves, a temporary MIR reads between two tests,
`if (j === 0 || v[i] !== w[j])`, was kept after the chain as `const value
= v[i];` where nothing read it (`returning_chain`): gone now, by the one
rule `and_tests` had, `logicals::unread_bound`.

## Why

- **The JS reads as upstream's**: `sampleLines[s]`, `sampleLines.length -
  1`, as React wrote them.
- **The program can't observe it**: each cast left out keeps the number,
  each check left out is one Rust's passes, and no Rust slice of sized
  items is longer than `isize::MAX`.
- **One answer for a cast that keeps its value**: the ranges' and the
  whole floats' are one set, asked in one place.

## Alternatives

- **Take every array's length as at most `2^31 - 1`**, as V8's are far
  below it: a `Vec<()>` may hold `usize::MAX` items in Rust, and its
  `len() as isize` is then negative. Rejected: results would change.
- **Read `s as usize` as `s` wherever `s` is an `isize`**: `-1 as usize`
  is `4294967295`, past the end, which panics; JS's `items[-1]` is
  `undefined`. Rejected.

## Consequences

- 17 corpus snapshots read as written: `v.length + 1` of a `Vec`'s
  length, `log.length - 8`, a `u8` loop's `i + 1` past an `i as u8`.
- `test/corpus/signed_index.rs` runs React's matching loop natively and
  in JS, a negative index's panic too.

## What holds it

- `test/lowering-numbers.test.ts`, "a signed index tested not negative
  is a JS index as it is", run in JS: the loop, an untested `i as usize`,
  an `isize`'s `- 1` that wraps, and a slice of `()`'s length.
- `test/lowering-written-branches.test.ts`, "a read a test takes in place
  is gone where nothing else reads it".
- Mutations `cast-kept-out-of-range`, `sized-length-of-zero-sized`,
  `join-unbound-forgets`, `widened-counter-forgets-bound`,
  `ne-length-not-below`, `assigned-operator-span-off`,
  `returning-chain-keeps-unread-bound`.
