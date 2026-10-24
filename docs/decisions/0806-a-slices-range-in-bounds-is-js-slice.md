# 0806. A slice's range in bounds is JS's `slice`, and a test held in a `bool` shows what it tests

Status: Accepted. Extends [0576](0576-what-a-bodys-integers-can-be.md)'s
ranges, which knew an index in bounds, to a range, and
[0746](0746-a-length-held-in-a-variable-shows-the-slice-so-long.md)'s
values held in variables to a test held in one.

Case ([0262](0262-when-rust-and-js-disagree.md)): A, the program can't
observe it. A range left unchecked is one whose check passes each time
it's reached; one the ranges can't show in bounds is checked as before.

## Context

React's `addValueToProperties` (shared/ReactPerformanceTrackProperties.js)
shows the first hundred items of a wider array:

```js
const didTruncate = array.length > OBJECT_WIDTH_LIMIT;
// ..
desc = JSON.stringify(
  didTruncate ? array.slice(0, OBJECT_WIDTH_LIMIT).concat('…') : array,
);
```

Its port, `let did_truncate = array.len() > OBJECT_WIDTH_LIMIT; .. if
did_truncate { [&array[..OBJECT_WIDTH_LIMIT], &[..]].concat() }`, read
`$slice(value, 0, OBJECT_WIDTH_LIMIT)`: `&v[a..b]` panics where `b`
passes the length, or `a` passes `b`, which JS's `slice` doesn't, so
rust-js checks each, `$slice`'s. Where `did_truncate` holds, it can't
panic, but the ranges knew neither:

- **A range wasn't judged.** The ranges judged an index, `v[i]`, in
  bounds where it's below the length each time it's reached (ADR 0576);
  a range, `Index::index(v, ..b)`, was always checked.
- **A test held in a variable showed nothing.** `if xs.len() >
  LIMIT { .. }` puts `xs` at least `LIMIT + 1` long in its branch; `if
  truncated { .. }` of `let truncated = xs.len() > LIMIT;` didn't.

## Decision

- **A slice's range in bounds is JS's own `slice`**: `&v[a..b]`, `&v[..b]`
  or `&v[a..]` of a `Vec`, a slice or an array, where `a ≤ b ≤ v.len()`
  each time it's reached, is `v.slice(a, b)` (`SliceLength::Range`,
  `judge_range`, read through `indexes_in_bounds` as an index is). `b ≤
  v.len()` by a bound by its length (`Below::up_to`), below its least
  length, or below an array's; `a ≤ b` of their ranges. Its value is an
  array made here, as `$slice`'s was (`Known::Sliced`, `made_array`), so
  `.to_vec()` of it is itself.
- **A `bool` variable made of a test that reads without changing anything,
  `let truncated = xs.len() > LIMIT;`, holds it** (`Known::tests`,
  `held_test`): a branch on the variable, `if truncated`, `!truncated`,
  shows what the test does there, while neither the variable nor anything
  the test reads has changed. What changes one forgets it: an assignment,
  a `&mut` given away, a `push` or a `pop` of the slice it reads
  (`changed`, `lent`, `grown`), and two paths keep it only where both
  hold the same test.
- **A test's sides are read once, as they were**: `value_of`, which a
  condition's sides and a range's ends are both read through, judging
  nothing.

```js
const didTruncate = value.length > OBJECT_WIDTH_LIMIT;
const kind = getArrayKind(value);
if (kind === "Primitive" || kind === "Empty") {
  desc = $unwrap(
    didTruncate
      ? JSON.stringify([...value.slice(0, OBJECT_WIDTH_LIMIT), "…"], null, 0)
      : JSON.stringify(value, null, 0),
  );
```

## Alternatives

- **`slice` always, as JS clamps**: a range past the end gives fewer
  items where Rust panics, a wrong answer. Rejected.
- **The test's facts kept where it's made, applied where it's tested**:
  what's known at the test may be narrower than where it was made, and
  what's known at the `let` may have changed by then; testing again, as
  written, where the branch is, while nothing it reads has changed, reads
  each side as it is there.
- **`RangeInclusive`, `a..=b`**: made by a call, not a struct, and none
  the ports use; checked as before.

## Consequences

- The guide's collections page has `head`, a slice in bounds after a
  test held in a `bool`.
- `test/corpus/slice_ranges_known.rs` runs each against native Rust, and
  `slice_range_tested_panic.rs` a test a `pop` made stale, still checked,
  panicking as Rust's does; `test/port-performance-track.test.ts`'s
  "addValueToProperties slices a truncated array as JS does"; mutations
  of `judge_range`, `up_to` and the tests' forgetting.
