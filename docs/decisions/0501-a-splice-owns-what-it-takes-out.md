# 0501. A `splice` owns what it takes out

Status: Accepted. Extends [0098](0098-destructors.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's dropped, and
when, as std's.

## Context

`v.splice(range, items)` takes out what `range` has, which its `Splice`
gives, and drops what it doesn't as it's dropped, as `drain` does; and
`inspect(f)` of an `Option` or a `Result` gives back what it's given. Of
values with a destructor, each was refused.

## Decision

**A `Splice` is an iterator over the array of what it took out, owning
each, as `drain`'s is (ADR 0098): its items its replacing iterator's,
`I::Item`, not the iterator. `inspect` keeps what it's given.**

## Why

- **It's std's**: `splice_and_inspect_drops` splices values with a
  destructor, dropping what's taken out at once, collecting it, and taking
  one then dropping the rest, and inspects an `Option` and a `Result` of
  one, with native Rust.

## Consequences

- **`extract_if` of values with a destructor** is refused yet.

## Amendment: a `Splice` dropped where it's made is its `$splice` (rd341, 2026-11-01)

`new_args.splice(at..at, [..]);`, its `Splice` dropped unread, printed
`$iter($splice(newArgs, at, at, [..]));`: an iterator of the array taken
out that nothing steps. `$splice` replaces the range as it's called, and
`$iter` of an array reads only what it's given, so **an array's iterator
made only to be dropped, a statement `$iter(items)`, is what making
`items` does, `$splice(newArgs, at, at, [..]);`** (tidy's `unread made`).
Case ([0262](0262-when-rust-and-js-disagree.md)): N. Held by the corpus's
`vec_edits` and `trailing_none_items`, run against native Rust, and the
mutation `unread-iter-kept` (`tidy/bindings.ts`).
