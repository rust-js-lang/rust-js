# 0151. Any function taken as a value is the arrow that calls it

Status: Accepted. Extends [0070](0070-function-values.md),
[0125](0125-function-values.md) and [0130](0130-if-let-values-and-fn-items.md).

Case: A, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A std function taken as a value was an arrow only where `std_fn_value`
listed its form, a JS method or a `Math` function of one argument (ADR
0070). Others were errors, though Rust code passes them where a closure
goes as often as it writes the closure:

```rust
let sizes: Vec<usize> = names.iter().map(String::len).collect();
groups.entry(key).or_insert_with(Vec::new).push(item);
let best = scores.into_iter().fold(0, i32::max);
rows.sort_by(i32::cmp);
```

`str::len`, `Vec::new`, `Option::unwrap`, `Result::ok`, `str::parse`, and
std's trait functions, `ToString::to_string`, `Into::into`, `Ord::max` and
`Ord::cmp`, each stopped a program. Each call compiled.

## Decision

**A function taken as a value that no other form fits is the arrow of its
parameters that calls it, lowered as its call would be:**

| Rust | JS |
|---|---|
| `.map(str::len)` | `.map((s) => $byteLen(s))` |
| `.or_insert_with(Vec::new)` | `$orInsertWith(groups, key, () => [])` |
| `.fold(0, i32::max)` | `.reduce((a, b) => Math.max(a, b), 0)` |
| `.sort_by(i32::cmp)` | `.sort((a, b) => $cmp(a, b))` |
| `.map(Option::unwrap)` | `.map((x) => $unwrap(x))` |
| `.map(str::parse::<u8>)` | `.map((s) => $parseInt(s, 0, 255))` |

- **The call is lowered in a copy of the body, given its arguments:** each
  is a variable that's one of the arrow's parameters. So the arrow is what
  `|a, b| i32::max(a, b)` would be, by the same lowering.
- **Its parameters are named as a closure's would be:** `s` of a string,
  `n` of a number, `x` else, or `a`, `b` of two and more. They're the
  arrow's alone, so the next arrow's can be `s` too.
- **A std trait's function** is the arrow too, where the crate's
  dictionaries don't have it (ADR 0130).
- **Still errors:** a function taking a `&mut` as a value, which would be
  given a place a parameter isn't (ADR 0099), and one whose call is an
  error.

## Why

- **It's exact:** the arrow runs what the call runs, the same lowering and
  the same panics. The `std_function_values` corpus case compares lengths,
  conversions, parsing, folds and a sort with native Rust, and
  `std_function_value_panic` the panic of `Option::unwrap`.
- **It's the JS a person writes:** `(a, b) => Math.max(a, b)`, as the
  closure was.
- **It's every function whose call compiles,** not a list kept beside the
  calls.

## Alternatives

- **More forms in `std_fn_value`.** Each would repeat its call's lowering
  for values, and miss the next function.

## Consequences

- A copy of the body is made for each such arrow, while it's lowered.
- `std_fn_value`'s forms stay, as they read the same as the call would.
