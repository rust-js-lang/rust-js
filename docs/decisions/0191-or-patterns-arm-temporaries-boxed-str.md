# 0191. An or-pattern's moves, an arm's temporaries, and `Box::from` of a `&str`

Status: Accepted. Extends [0063](0063-text.md), [0098](0098-destructors.md)
and [0131](0131-temporaries-taken-apart.md).

## Context

chrono stopped at its last four places, after ADR 0190:

- **`Single(t) | Ambiguous(t, _) => Some(t)`,** twice, in
  `MappedLocalTime::earliest` and `latest`: an or-pattern that moves part
  of a value with a destructor, `T`, and leaves the rest to drop.
- **`Ambiguous(min, max) => match (f(min), f(max)) { .. }`,** in its
  `and_then`: a temporary in an arm's body, which ends with the arm.
- **`Box::from(s)` of a `&str`,** in `Item::to_owned`.

## Decision

- **An or-pattern moves what each alternative moves, where whichever
  matched, what the others would move isn't there:** each part's flag is
  cleared, as one alternative's is (ADR 0098), and the drop at the end of
  the value's scope tests its variant before its flag:

  ```js
  mapped$Single$0$live = false;
  mapped$Ambiguous$0$live = false;
  const t = mapped._0;
  ```

  So what one alternative moves that another doesn't must be of a variant
  the other excludes. `(Some(t), _) | (_, Some(t))` of a pair of two, which
  both may match, is still an error: which moved is the value's to know.
- **A temporary in an arm's body ends with the arm,** as rustc's scope
  tree says, dropped before what the arm binds. A pair whose first item is
  dropped should the second's call panic is owned once both are made: its
  `try` begins after the one that drops the first.
- **A `Box<str>` from a `&str` or a `String` is the string,** as a `String`
  from one is, and a `Box<str>` is one already.

## Why

- **It's exact:** the `or_pattern_part_moves` corpus case moves an end out
  of an ambiguous time and a part out of each alternative of `if let` and
  `let else`, with native Rust's drops; `arm_temporaries` maps chrono's
  `and_then` over results with destructors, one of which is dropped, and
  `boxed_str` makes, compares, shows and clones `Box<str>`s.
- **chrono compiles,** its whole crate.

## Costs

- **An or-pattern whose alternatives move parts no variant tells apart is
  still an error.**
- **A temporary assigned in an earlier one's `try` with what may leave
  after it there is still an error,** a guard no program found makes so.
