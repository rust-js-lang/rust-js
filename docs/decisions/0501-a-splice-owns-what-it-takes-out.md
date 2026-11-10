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

## Amendment: a `Splice` puts its replacement in place as it's dropped (#1147, #1149)

`$splice` stepped the replacement whole as it was called, and gave the
array of what it took out, a `Drain`'s form. Std's `Splice` takes the
range out where it's made, and its drop puts the replacement in place:
what it hasn't given dropped, then the replacement stepped into the gap,
each item written as it's stepped; then as many as its hint's lower bound;
then what's left collected apart and moved in; then the tail moved back
(`Drain::drop`) and the replacement dropped, the `Splice`'s fields in their
order, as it ends or unwinds. So `v.splice(1..2, repeat_n(D(7), 2))`
dropped at once printed `clone 7` before `drop 11`, which native Rust
prints after; a replacement whose step panicked left the `Vec` as it was
and leaked what it had stepped, where native Rust keeps what it put in the
gap (#1149); and the crate's own iterator given as the replacement was
read as its struct, an item `undefined` (#1147).

**A `Splice` is a JS iterator over what it took out, `$splice`'s
`$closing`, whose close runs std's drop**, faithful to its steps: the gap
filled as each is stepped, the hint's lower bound filled, the rest
collected apart and dropped where a step panics, the tail put back and
the replacement closed, though one throws. It's done with as an owning
iterator is: stepped to its end, a `collect()`'s `Array.from`, or dropped,
`s.return()`, which a `Splice`'s drop glue is, whatever its items, as
its drop always puts its replacement in place (`drops`'s and
`drop_parts`' `is_vec_splice`). It knows its length and its back. The
replacement is the crate's own iterator stepped by its `next`
(`iterable_items`), and of one that holds what drops, closed to drop it.
`$splice` is one of the runtime's iterator helpers, and it's no longer
an array source.

The 2026-11-01 amendment's statement `$splice(..)` is
`$splice(..).return()`, which drops it there.

Held by `test/corpus/splice_drops_and_fills_as_std.rs`, against native
Rust in both builds (collected, held and dropped, asked its `len()` and
stepped from its back, dropped at once, of the crate's iterator, of
clones, of steps that panic past the gap and of a filter whose hint is no
help, at the `Vec`'s end, and generic code's), and
`runtime_long_writes.rs`'s replacement collected apart past its hint.
`vec_edits`, `trailing_none_items`, `runtime_long_writes`,
`consumer_closes_what_it_steps_on_a_panic` and `splice_and_inspect_drops`
take its new form. Mutations `splice-removed-undropped`,
`splice-gap-collected-apart`, `splice-tail-lost-on-a-panic`
(`runtime/splice.ts`), `splice-dropped-unclosed` (`lower/drops/parts.ts`),
`splice-drop-of-its-items` (`lower/drops/types.ts`),
`splice-length-unknown` and `splice-backless`
(`lower/iterators/knowing.ts`) new; `splice-replaces-nothing`,
`splice-replacement-spread`, `splice-replacement-unclosed`,
`owned-iterator-item-first-arg`, `drain-not-owning` and
`memchr-iter-backless` refitted; `splice-items-its-iterator` and
`splice-unowned`, whose sites are gone, removed.

## Amended (2026-11-10, #1189): put in place where it's made, where nothing it does runs code

#1149 made every `Splice` std's lazy one, `$splice(..)`'s iterator whose
drop puts the replacement in place, and a statement `$splice(..).return()`.
That's needed where it can be seen: a replacement's steps that run code
or panic, or items with a destructor, whose drops std orders around those
steps. Where none can, it's noise: React's port, whose sites splice arrays
of strings and of values with no destructor, took `.return()` at ten sites,
and upstream React's JS is `array.splice(..)` alone.

**A `Splice` of an array's items with no destructor, its replacement an
array's, a `Vec`'s or an array's `IntoIter`, a slice's `Iter` or its
`copied()`, drops nothing (`drops`, `Recognition::splice_of_array_items`),
and is put in place where it's made, `$splice(v, a, b, items)`**, which
gives a `$iter` of what it took out, collected, reversed or counted as
std's `Splice` gives it. Nothing sees the `Vec` till the `Splice` is
dropped, as it borrows it, and stepping such a replacement runs nothing
and can't panic, so where it's put in is unseen (case A,
[0262](0262-when-rust-and-js-disagree.md)). Every other `Splice` is std's,
now `$splicing`. The one answer is `drops`': the call is `$splice` where
its `Splice`'s drop is `Drops::Nothing`. `splice_of_array_items.rs` holds
the eager form against native Rust, its result used each way; the port
regenerates at its pin with no diff, its Fizz site's comment on its
statement again.
