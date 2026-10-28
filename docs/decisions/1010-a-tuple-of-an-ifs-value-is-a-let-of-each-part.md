# 1010. A tuple of an `if`'s value taken apart is a `let` of each part

Status: Accepted. The user's decision (2026-10-28), React port gap rd45.

## Context

Upstream's trackHydrated (inputValueTracking) declares two variables and
sets both in each branch:

```js
let valueField;
let expectedValue;
if (isCheckable(node)) {
  valueField = 'checked';
  expectedValue = '' + node.checked;
} else {
  valueField = 'value';
  expectedValue = node.value;
}
```

Rust's late-init form of it, `let a; let b; if .. { a = ..; b = ..; }`,
is refused by clippy's default `needless_late_init`, which asks for a
tuple of the `if`'s value, `let (value_field, expected_value) = if
checkable { ("checked", ..) } else { ("value", ..) };`. That printed an
array destructuring of a conditional of two array literals, `const
[valueField, expectedValue] = checkable ? ["checked", ..] : ["value",
..];`: two arrays upstream doesn't make, and not what a person writes.

## Decision

**A `let` that takes apart a tuple of an `if`'s or a `match`'s value,
each branch's a tuple written out, is a `let` of each part, assigned in
each branch** (tidy's `conditional_tuples`, `destructuring.rs`, after the
destructuring passes): `let a; let b; if (c) { a = x; b = y; } else { a
= z; b = w; }`, an `else if` chain of an `if`'s, as written. Each part
is given in its order, as the array's items were made, and nothing in a
branch reads the variables, declared there. A branch whose value isn't
a tuple written out, `if c { pair() } else { (1, 2) }`, keeps the array
destructuring; so does a tuple kept whole, `let p = if c { (1, 2) } else
{ (3, 4) };`.

Case ([0262](0262-when-rust-and-js-disagree.md)): None: the same values
given in the same order, and the variables nothing reads before.

## Consequences

The playground's `const [outputPaths, shown] = output.TAG === "Files" ?
[..] : [[], ""];` is two `let`s assigned in each branch. Held by
`test/corpus/tuple_of_an_if_taken_apart.rs` against native Rust, with the
order each branch's parts are made in logged, and the mutations
`conditional-tuples-kept-whole` and `conditional-tuples-else-dropped`
(`lower/mir/tidy/destructuring.ts`).
