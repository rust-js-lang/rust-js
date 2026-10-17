# 0431. std's conversions are one list of what's converted into what

Status: Accepted. Extends [0063](0063-text.md),
[0023](0023-strings-references-shared-state.md) and
[0320](0320-counted-rcs.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)): each conversion's JS
value is the one Rust's makes, the same JS value where Rust's moves it.

## Context

`classify_conversion` read a `From` and an `Into` apart: what `From`
converted from was its argument, into its type; `Into` the other way, and
each had its own rules for what it knew. The two had drifted:
`[8, 9].into()` into a `Vec` was known, `Vec::from([8, 9])` refused. And
common conversions were in neither: `Option::from(x)`, `Rc::from(x)`,
`Rc<str>::from("hi")`, `Arc<[T]>::from(vec)`.

## Decision

**What a conversion converts into what is `conversion_sides`', the one
answer of which side is which, of `From`, `Into`, `TryFrom` and `TryInto`;
and std's `From`s are one list of rules over the two sides, which the
`Into` each gives reads alike.**

- **`Option::from(x)`** is `Some(x)`, `Std::OptionFrom`, boxed where it
  could look like `None` (ADR 0051).
- **A `Vec` of an array**, `Vec::from([..])` as `[..].into()` was.
- **An `Rc` or an `Arc` of a value** is `Rc::new(x)`'s, the value itself,
  or counted where its counts are read (ADR 0320): of the value; of a
  `str`, text, a `&str`, a `String` or a `Box<str>`; of a `[T]`, a
  `Vec<T>`'s or a `Box<[T]>`'s items, moved. Which is
  `recognition::rc_made_of`, which `rc_same`, what says an `Rc` is made,
  asks too.
- **A `Cow`** of a reference to what it borrows, or to what owns it, is
  `Borrowed`, `{ TAG: "Borrowed", _0: s }`, and of what owns, `Owned`,
  `CowOp::From` (ADR 0319).

## Why

- **It's one list**: a rule is written once, for either way it's asked.
- **It's tested**: the `std_conversions` corpus case converts each way,
  with native Rust, and `rc_counted_uses` counts an `Rc::from`.

## Consequences

- A conversion this list doesn't have is still refused, as it was.
