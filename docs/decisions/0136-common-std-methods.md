# 0136. Common std methods: `take`, `cmp::max`, ASCII case, and `Debug`'s builders

Status: Accepted. Extends [0054](0054-display.md), [0060](0060-debug.md) and [0074](0074-mut-boxes.md).

Case: C, A, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

Some std methods that ordinary Rust calls all the time were errors:

- an `Option`'s `take()` and `replace(v)`, and `mem::take(&mut x)`;
- `cmp::min(a, b)` and `cmp::max(a, b)`, as functions;
- `to_ascii_lowercase()`, `to_ascii_uppercase()` and `eq_ignore_ascii_case`
  of a `str`;
- an iterator's `inspect(f)`, a `Result`'s `as_ref()`, a `Cell`'s and a
  `RefCell`'s `into_inner()`, a `Vec`'s and a `VecDeque`'s `append`, and a
  float's `exp2()`, `exp_m1()` and `ln_1p()`;
- a hand-written `Debug`'s builders: `f.debug_struct("P").field("x",
  &self.x).finish()`, and `debug_tuple`, `debug_list`, `debug_set` and
  `debug_map`.

## Decision

| Rust | JS |
|---|---|
| `let t = o.take();` | `const old = o; o = undefined; const t = old;` |
| `o.replace(5)`, `mem::take(&mut v)` | as `mem::replace` with `Some(5)`, and `v`'s default (ADR 0098) |
| `cmp::max(a, b)` | `a.max(b)`'s: `Math.max(a, b)`, or `Ord::max`'s |
| `s.to_ascii_lowercase()` | `$asciiCase(s)`: ASCII's letters only |
| `a.eq_ignore_ascii_case(b)` | `$asciiCase(a) === $asciiCase(b)` |
| `iter.inspect(f)` | `iter.map((item) => { f(item); return item; })` |
| `r.as_ref()` of a `Result` | `r`: a reference is the value (ADR 0023) |
| `cell.into_inner()` | `cell.value` |
| `v.append(&mut other)` | `$append(v, other)`, which empties `other` |
| `x.exp2()`, `x.exp_m1()`, `x.ln_1p()` | `2 ** x`, `Math.expm1(x)`, `Math.log1p(x)` |
| `f.debug_struct("P").field("x", &self.x).finish()` | `` `P { x: ${p.x} }` ``, written as one string |
| `f.debug_list().entry(&0).entries(v.iter()).finish()` | `` `[${["0"].concat(Array.from(v, ..)).join(", ")}]` `` |

- **`take` and `replace` write the place**, as `mem::replace` does, through
  a box for a `&mut` to one (ADR 0074), and give back the old value, boxed
  already where a generic one is (ADR 0051). Of a value with a destructor,
  which the old one would need dropping, they're still errors.
- **`cmp::max` is `Ord::max`**: of numbers, `Math.max`, and of anything else,
  the order rust-js compares it by, as the method's is. Equal, it's the
  second, as Rust's is.
- **ASCII case changes only ASCII's letters**, as Rust's does: `Ü` stays.
- **A `Debug` builder's chain is one string**, what a derived `Debug`
  writes: its `&dyn Debug` arguments are the strings they show already (ADR
  0060), and `entries` an array of them, joined with the rest. A builder
  kept in a variable and written to in turn, and `{:#?}`, are still errors.
  (Amended: both compile, ADRs [0137](0137-pretty-debug.md) and
  [0532](0532-a-debug-builder-held-in-a-variable.md).)

## Why

- **It's the JS a person writes**: a write and a read, a string of the
  fields, `2 ** x`.
- **It's exact**: what each gives and changes is Rust's, compared with native
  Rust by the `std_methods` corpus case.

## Consequences

- These compile.
- `by_ref()`, which shares an iterator's place with a later use, is still an
  error.

## Amendment: `clamp` of what JS's `<` doesn't order

`clamp` of text, a `char`, or a type of the crate's or std's whose `Ord`
isn't a number's was refused (#118); `max` and `min` of them were
`OrderingCall`s already. `clamp` is one too: `$clampBy(x, min, max, cmp)`
by its `Ord`'s `cmp`. Bounds the wrong way round panic as `Ord`'s default
does, `assertion failed: min <= max`, and of a `char`, as its own `clamp`
does, showing them: `min > max. min = 'k', max = 'a'`. Held by
`test/corpus/clamp_ord.rs` and `test/corpus/clamp_char_bounds.rs`.

## Amendment (2026-10-24): six more of strings' odds and ends

Case ([0262](0262-when-rust-and-js-disagree.md)): R, results change; each
was refused (#118).

- **`fmt::Error`'s `{}`** is std's text, "an error occurred when formatting
  an argument".
- **A `ParseIntError`'s `clone()`** is itself: what it's made of never
  changes, as a path's text doesn't.
- **`Chars`' `{:?}`** is `Chars(['b', 'c'])`, the `char`s it has left, as
  std shows it. Its `{:#?}` stays refused.
- **`CharIndices::offset()`** is where its next `char` starts, in UTF-8
  bytes, or the string's length past its last: `$charIndicesOffset`.
- **`c.encode_utf16(&mut buf)`** writes its units at `buf`'s start and
  gives the `&mut [u16]` of them, panicking with std's text where `buf` is
  too short: `$encodeCharUtf16`.
- **A string `Drain`'s `as_str()`** is what it has left: the `char`s it
  took out are an `$iter`, as `chars()`' are, so `$restStr` reads it. Its
  `next_back` stays refused.

Held by `test/corpus/str_odds.rs` against native Rust, and the mutations
of each file changed. `String::capacity` stays refused: what it gives is
the allocator's.
