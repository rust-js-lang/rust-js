# 0986. `len()` of an iterator made in place is counted through its adapters

Status: Accepted; its reading of a map of a `rev()`'s items, which read
a reversed range forward, is corrected by
[0987](0987-a-rev-made-in-place-read-by-its-form.md). A gap closed, found by #382's triage: `len()` of an
`ExactSizeIterator` made where it's asked, `v.iter().map(f).len()`, was
refused.

Case ([0262](0262-when-rust-and-js-disagree.md)): a gap, valid Rust
refused. Its result follows Rust's: `len()` steps nothing and calls no
adapter's function.

## Context

A std adapter held in a variable is std's compiled struct, whose
`ExactSizeIterator::len` is std's (ADR 0871). An adapter made where
`len()` is asked is what the recognizers make in place: `$iter(v).map(f)`,
`$iterator(range, $rangeNextBack)`, `$taking(..)`. Its `len()` had rules
for an array's items, a range and an array's skipped or taken items alone.
Each of these was refused, though each is an `ExactSizeIterator`:

```rust
v.iter().map(|x| x * 2).len()
(1..4u32).rev().len()
v.iter().enumerate().rev().len()
v.iter().take(2).map(|x| x + 1).len()
fn taken<I: ExactSizeIterator>(it: I) -> usize { it.take(2).len() }
```

So was a `rev()` of a map, `count(v.iter().map(f).rev())`, given to
generic code that asks its length (`as_exact_size`).

Counting by stepping, `.toArray().length`, would call each map's function,
which Rust's `len()` never does. ADR 0036 refused `len()` of a chain
whose closures do what can be seen for that reason; this decision amends
it, and its diagnostics test, "len of a chain whose closure prints", is
removed.

## Decision

`len_in_place`, beside `items_left` in `iterators/mir_steps/sources.rs`, counts an
iterator made in place by its type, as std's `len()` of each adapter asks
its source's, reading each value's form through the recognizers'
readers:

- a `map`, an `enumerate()`, a `copied()` and a `rev()`: its source's,
  where what the map is given does nothing to make;
- `take(n)`: `Math.min(len, n)`; `skip(n)`: `Math.max(len - n, 0)`, its
  source's count read before `n`, where `n` does nothing to read;
- an array's items, `$iter(xs)`: `xs.length`;
- a range of numbers: its count, as before;
- a generic one or a `dyn`'s, given knowing its length (ADR 0456): its
  own, `it.len()`, not `Iterator.from(it)`'s.

Any other form is refused, as before. `len()`'s rule of a range is this
one's.

`as_exact_size` makes a `rev()` made in place knowing its length:
- `$rev(source)`, and a range's stepped from its back: `$rev` of its
  source made knowing its back and length;
- an array's items reversed, `$iter(xs.toReversed())`: as it is;
- a map of them: as its source's map is.

A `rev()` a dictionary steps from its back, `$iterator(it, nextBack)`,
knows neither, and stays refused.

## Consequences

`v.iter().map(f).len()` is `v.length`, and `v.iter().skip(k).map(f).len()`
is `Math.max(v.length - k, 0)`. `len()`'s rule for an array's items skipped
or taken from, `counted_chain`'s `v.slice(9).length`, counted the same
construct a second way. It's removed, with its mutation,
`mir-counted-chain-len-refused`: `v.iter().take(9).len()` is
`Math.min(v.length, 9)`, as `take(2).map(f)`'s is (`len_made_in_place`,
`str_bytes`).

`test/corpus/len_made_in_place.rs` checks each form against native Rust,
including a map whose function counts its calls. The mutations are
`len-in-place-*` and `exact-size-rev-unread`,
`exact-size-reversed-items-unread` and `exact-size-reversed-map-unread`.
The case also catches #382's `rev-length-unknown`.

## Amendment: `size_hint()` of one counted in place (#156)

`v.iter().skip(1).size_hint()` gave `(0, None)`, a JS helper's
`$sizeHint`, where Rust gives `(4, Some(4))`. What `len_in_place` counts
is an `ExactSizeIterator`'s length, so its hint is that both ways, `const
n = Math.max(v.length - 1, 0); [n, n]`. Not an array's items, or a map of
them, whose `$iter` knows its hint already, so `size_hint.js` reads as it
did. `test/corpus/size_hints_made_in_place.rs` checks `skip`, `take`,
both, and a map against native Rust. The mutation is
`size-hint-in-place-uncounted`.

## Amended (2026-11-06, #811): a `step_by(n)` of any source counted in place

`len_in_place` counted a `step_by(n)` of a range alone, `$stepBy(range,
n)`, so `v.iter().step_by(3).len()` was refused. **It counts a `step_by(n)`
of either form, `stepped_by_parts`' (stepping.rs, the one reader of a
filter of every `n`th and of `$stepBy`), by its source's count:
`Math.ceil(v.length / 3)`**, a skip's or a map's of a slice's items too,
and of `step_by(1)` the count itself. A `rev()`'s source, a `step_by(n)`
made knowing its back, `$stepByEnds(src, n)` (value.rs' `stepped_by_ends`,
which alone writes it), is read as one too: `v.iter().step_by(3).rev().len()`.
Not yet one of a map, `map(f).step_by(n).rev()`, whose source is `$mapEnds`
(diagnostics' "len of a reversed step_by of a map").

What collects one asks room of a count no array bounds (ADR 0940), not of
a `step_by(n)` of an array's items, which no array's length overflows:
its eager filter stays as it was. A reversed range's `step_by(n)`, which
`len_in_place` now counts, asks it as std's `SpecFromIter` does, so
`(0..=u32::MAX).rev().step_by(1 << 30).collect()` reads `$roomFor(Math.ceil(
4294967296 / step), 4)` (`range_steps_known`), and a range a capacity
can't hold panics `capacity overflow` as Rust's does.

Case ([0262](0262-when-rust-and-js-disagree.md)): none observable but the
room a range asks, which Rust asks too. Held by corpus
`generic_and_counted_steps_from_their_back`; `collected_counts_ask_room`'s
`step_by(1)` reads its count. Mutations `step-by-len-of-items-uncounted`,
`step-by-of-one-divided`, `step-by-of-items-asks-room`, `step-by-ends-unread`;
`mir-len-closure-type-shown` asks the diagnostics row above; `step-by-uncounted`,
`step-by-made-unstepped` and `step-by-made-by-nth-backless` name their
lines as they now read.

## Amended (2026-11-12, #1483, #1485): each part runs once, in its order

A count read in place left out what it didn't read and could read a part
twice. `len()` and `size_hint()` of `repeat_n(made(2), 3).enumerate()`
counted `3` and never ran `made(2)`. `(made(5)..made(9)).step_by(2).len()`
ran `made(9)` first, `Math.max(made(9) - made(5), 0)`. And `eq` of two
`TrustedLen` iterators (ADR 0992) read each length, `Math.max(made(4) -
made(1), 0)`, before it made the iterator of the same text, so the bounds
ran twice. Two deciders answered whether a count runs code:
`counted_in_place` checked only the counts it read itself, and
`collected_counted` checked the whole answer again (#1441). That second
check covered every case of the first, so mutation
`count-read-through-a-call-read-again` survived the nightly.

**`counted_in_place` takes `discards`**, set where the iterator isn't made
afterwards (`len_in_place`, `len()`'s and `size_hint()`'s). Of such a
count it refuses (`Uncounted::RunsCode`) where what it leaves unread runs
code: `repeat_n`'s `x` and what it clones and drops with, a `map`'s
function, or an endless source a `take(n)` counts by `n`. A range's start
is held first where both bounds run code, so each runs in its order. A
count it reads, `*held.as_ref().unwrap()`, is read once, so
`repeat_n(1u8, *held.as_ref().unwrap()).map(f).len()` now compiles. It
was refused. **What reads a count before making the iterator of the same
text asks `unless_run_again`**: a collect's room, and an `eq`'s lengths.
That is the one answer of whether reading the count runs code.
`repeat_n(made(2), 3).enumerate().len()` is refused now; it was wrong.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change
(evaluation order and side effects), so Rust's are kept. No snapshot
changed: every corpus case compiles to the same JS in both builds. Held by
corpus `counted_where_made_runs_each_part_once`. Mutations
`count-of-a-call-read-again`, `count-leaves-unread-code-out`,
`range-count-reads-its-end-first` and `eq-length-of-a-call-read-again`.
`count-read-through-a-call-read-again` and collect.rs's
`collect-count-of-a-call-read-again` are gone with the leaf checks and the
caller's copy.

## Amendment (2026-11-17, #1978): a map made knowing its back, and a clone that's its source

A `rev()` of a `copied()` of a slice's `map` makes the map one that knows
its back, `$mapEnds($iter(items), f)`, where a `rev()` straight over the map
reverses the array inward. The count read only JS's own `source.map(f)`,
so `len()` of `items.iter().map(f).copied().rev()` was refused. A map's
parts are now read by one reader, `mapped_parts` (iterators/value.rs,
which owns the forms): `source.map(f)` or `$mapEnds(source, f)`. Either
is counted as its source, `f` made and not called: `items.length`.

A `cloned()` of what needs no clone is its source in JS, as a `copied()`
is (`is_its_source`, ADR 0547), and is now counted as its source too.

A `copied()` or `cloned()` of a map held in a variable whose `len()` is
asked stays refused. In place, the copy's map is `$mapEnds(..)` or JS's
own `.map(f)`, not std's compiled `Map`, and what asks a held value's
steps by its type, a compiled `Rev`'s dictionary of what it holds, can't
tell which. Reading it as its source's type gave the compiled `Map`'s
`len` to a `$mapEnds` and threw, so it stays refused, #1978's remainder.

The same reader counts what a `rev()` made so elsewhere, each refused
before:
- an `enumerate()`, a `step_by(s)` or a `skip(k)` of a slice's map
  reversed;
- a slice's `chunks(k)` mapped;
- a `take(k)` of a reversed copy.

A collect of such a count asks no hint, `$roomForHint`, any more, as one
counted where it's made doesn't (`back_steps_beside_a_live_drop`'s
snapshot). The diagnostics row that held a refused `len()` naming its
adapter's type, once a `step_by(2).rev()` of a map, is now a
`fuse().rev()` of one, still refused.

Corpus case `len_of_a_reversed_copy_of_a_map` holds:
- a `copied()`'s, a `cloned()`'s and a `skip(1)`'s count;
- a map whose function is never called;
- a `map(..).rev()` collected and a reversed sum;
- each shape above, at each length to 6 and a step known only at run time.
It matches native in each build and through every metamorphic rewrite. Mutations
`map-ends-parts-unread` and `clone-as-its-source-uncounted` are caught.

Rows of generic exact-size code (#1979) hold a generic iterator's own
`len()` where a zip of it with a slice's items, or a `take(n)` or `skip(n)`
of its copies, counts it in place. They make `len-in-place-generic-unread`
and `len-in-place-generic-wrapped` caught. docs/readability.txt is blessed
with them. The check failed only at these rows, `$iter`'s share past its
84.9 in 10,000 lines, and the baseline now reads 85.5, its rows caught up
with the cases landed since its last bless. The growth comes only from
new programs, these rows' generic code lifting a JS iterator by `$iter`
as this ADR requires. Running the check over the stack's snapshots one
commit at a time showed no existing program's output got worse.

## Amendment (2026-11-17, #1978): a held copy of a map whose length is asked

The amendment above left one shape refused: a `copied()` or `cloned()` of
a map, held in a variable, whose `len()` is asked. The copy is its map in
JS, made in place, and the type says nothing of its form. The map is
`$mapEnds(..)` where its back is asked and JS's own `.map(f)` otherwise.
Back_stepped's `asked_of`, the one answer of what's asked of a local, now
answers it in two places.

- **A `len()` of a variable is a length asked of it.** Through what moves
  or lends it, the variable is `LengthAsked::Held`, as a `Vec`'s growth
  asks (#1795). `written_as_asked` makes its value knowing its length
  where it's written: `const m = $mapEnds($iter(items), f); m.len()`.
- **A `rev()` a variable holds is std's struct (ADR 0871).** It steps what
  it holds from its back and asks it its length by the dictionary of its
  type alone, whatever the variable is asked. Where what it holds isn't
  std's compiled struct, a copy of a map made in place, that type and
  each adapter beneath it that steps from its back where its source does
  are made knowing their back by type (`typed_sources`, as #851 made the
  source of a struct stepped from its back). The slice's iterator beneath
  isn't, as other values share its type: `iterator_rev($mapEnds($iter(items),
  f))`, whose `len` is the map's own.

No router changed: a call on a value made in place already goes to the
iterator lowering (`steps_adapter_in_place`). Nothing else printed moved:
the whole corpus and std's structs ran unchanged, snapshots compared.

Corpus case `len_of_a_held_copy_of_a_map` holds such a copy, reversed or
not, against native in each build and through every metamorphic rewrite:
- its length alone;
- stepped from either end, then its length;
- its hint, and given to generic code;
- items with a destructor, cloned, collected and dropped.

Mutations `held-rev-in-place-source-untyped` and `len-of-a-variable-unasked`
(`analysis/back_stepped.ts`) are caught. docs/readability.txt is blessed
with the case: `$iter`'s share goes from 85.5 to 85.8 in 10,000 lines,
only from the case's own ten, each the source of a map this ADR counts.
No existing program's output changed.
