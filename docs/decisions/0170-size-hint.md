# 0170. `size_hint()` where its answer is known

Status: Accepted. Extends [0164](0164-double-ended-iterators.md). Narrowed
by [0989](0989-a-rev-hints-what-its-source-hints.md): a `rev()` of what
generic code steps hints what its source hints.

Case: B, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

An iterator that wraps another passes its hint on, as utf8_iter's
`Utf8CharIndices` does:

```rust
fn size_hint(&self) -> (usize, Option<usize>) {
    self.iter.size_hint()
}
```

`size_hint()` of anything but the crate's own impl was an error, which
stopped utf8_iter, so url.

## Decision

**`size_hint()` is Rust's answer where rust-js knows it, bounds that hold
of a generic iterator, and an error elsewhere:**

| Of | JS |
|---|---|
| an iterator of the crate's that keeps std's | `[0, undefined]`, std's `(0, None)` |
| one with its own | its impl's, as before |
| std's that's an `ExactSizeIterator` over an array, `v.iter()`, `into_iter()`, `map` of one | `[n, n]`, its `len()` (ADR 0164) |
| a generic `I: Iterator`'s | `$sizeHint(it)`: exact where it steps through an array, as `$iter` gives one; `[0, undefined]` of a lazy one. (Amended: an array itself too, which a generic iterator never is; and an associated type's, smallvec's `I::IntoIter`, alike, `test/corpus/size_hint_into_iter.rs`.) |

- **A generic `I: Iterator`'s is a difference, listed:** rust-js doesn't
  keep its type (ADR 0061), so the hint is what its JS value tells. An
  array's is exact, where Rust's may be wider: an eager `filter` is an
  array of what it kept, whose Rust hint is `(0, Some(n))`. A lazy one's,
  or one of the crate's with its own `size_hint`, is `(0, None)`. Rust
  lets a hint be any bounds that hold, and these do; serde_core's
  `iterator_len_hint` sizes a buffer by it. (Amended: an error, which
  stopped serde_core; accepted as a difference.)
- **Still errors:** std's whose hint isn't exact, as `chars()`'s isn't, and
  a range's.

## Why

- **It's exact:** an `ExactSizeIterator`'s hint is its length, which std
  promises; the `size_hint` corpus case compares the crate's iterators, one
  passing another's on, and std's partly stepped, mapped and owned, with
  native Rust; a diagnostics test the refusal of `chars()`'s.
- **It's the JS a person writes:** a length, twice.

## Amendment: a range given as an `Iterator` knows its hint

A range given as an `Iterator` to generic or std code, std's `Peekable`
(ADR 0561), is the range object with `{ next: $rangeNext }`, and
`$sizeHint` found no hint of it: `(0, None)`, and `len()` panicked on
its `debug_assert` (#111). `$sizeHint` of what's given with `$rangeNext`
as its `next`, a range of numbers, is exact now, as `$rangeLen` counts
it; the dictionary stays `{ next: $rangeNext }`, as nothing else asks
it. ADR 0262's case: `len()` panicked, so Rust's. Held by
`test/corpus/peekable_range_len.rs` and the mutation
`size-hint-of-range-unknown`.

## Amendment: a `map` of the crate's iterator passes its hint on

`size_hint()` of a `map` of the crate's iterator was refused: a `map`'s
hint was known only of an array's `$iter` under it. Std's `Map` passes
its source's on, so it is now, made in place or held:

- `size_hint_of` peels each `Map` off the type as it peels each `map`
  off the JS, and asks the crate's `$iterator` under them, which has its
  impl's own `size_hint`, or gives std's `(0, None)`.
- A `$Steps`'s `map` has a `hint`, its source's, so one held in a
  variable is asked it too (`maps_the_crates_own`).
- The `$iterator` a `map` steps keeps its `size_hint`, which
  `consumed_into_iters` dropped as unasked.

ADR 0262's case: an error where Rust is fine, so Rust's. Held by
`test/corpus/adapter_size_hints.rs` and the mutations
`map-hint-of-own-refused`, `held-map-hint-refused`, `map-hint-lost` and
`map-source-hint-trimmed`.

## Amendment: a string's `chars()` hints as std's (#1186)

`size_hint()` of a `chars()` or `char_indices()` itself is std's,
`(bytes.div_ceil(4), Some(bytes))` of the bytes it has left. It was
refused while `chars()` was an array whose count was exact. A crate that
asks it makes each `Chars` the runtime's `$chars(s)`, which knows it
([1117](1117-a-strings-chars-are-lazy.md)). Corpus case
`chars_size_hint.rs`.

## Amendment: a `take` or a `filter` of a type parameter's hints as std's (#1236)

Case ([0262](0262-when-rust-and-js-disagree.md)): results changed, now
Rust's. Generic code's `i.take(2).size_hint()` of an `I: Iterator` made
`Iterator.from(i).take(2)`, JS's own adapter, which knows no hint, so
`$sizeHint` gave `(0, None)` where std's `Take` gives `(2, Some(2))` of a
`Vec`'s `into_iter()`; `size_hint()` was taken for known, `Take` a compiled
std struct of what it holds, which this `take` made here isn't. A `take(n)`
or a `filter(f)` made where its hint is asked, of an iterator `$sizeHint`
asks, a type parameter's given as a JS iterator, is std's of its source's,
`$takeHint(i, 2)` and `$upperOnly(i)`, so a generic `filter`'s, refused
before, is std's too. Found by the audit of #1170's migration 5. Held by
`take_and_filter_of_a_type_parameter_hint_as_std.rs` and the mutation
`in-place-adapter-hint-unasked`.

## Amendment: what generic code asks the hint of is made knowing it (#1236)

Case ([0262](0262-when-rust-and-js-disagree.md)): results changed, now
Rust's; the user's decision, A1, over a refusal. A std adapter a caller
makes and gives generic code that asks its hint, `hint(v.iter().filter(p))`
of `fn hint<I: Iterator>(i: I) { i.size_hint() }`, was JS's own,
`values().filter(p)`, which knows no hint: `(0, None)` where std's is
`(0, Some(4))`. The caller didn't know its callee asks it. Now a body's
`size_hint()` of a type of its type parameters asks it of each instance
its callers give (`hint_asked.rs`, the callers' instances' carrier's
`seen_instances_of`, `Fact::Marked`: each caller seen marks what it
gives), and a library's generic function's iterators, whose body the
crate can't see, are asked so too. Of what steps an array's items, whose
JS, `values()` and its own adapters, knows none, each is made in
rust-js's form, which knows std's hint as its back: a map's or a filter's
`$mapEnds` or `$filterEnds` of `$iter(v)` (the back's own makers, `typed`),
a `take(n)`'s, a `skip(n)`'s or a `step_by(n)`'s `$taking`, `$skipping`
or `$stepBy` (`hinted`). A range's, the crate's own iterator's and std's
compiled ones are `$Steps` already, whose own adapters know it. The fact
is the "made knowing" machinery's beside the back's, and #1180's capacity
questions can ask it too. Held by
`std_adapter_given_to_generic_code_that_asks_its_hint.rs`, the crate pair
`library_hint_asked_of_what_its_given`, and the mutations
`hint-asked-unmarked` and `library-callee-hint-unmarked`.

Still `(0, None)`: an iterator JS gives, a JS iterator's, which knows no
hint, the guide's difference 14.

## Amended (2026-11-10, #1244): a hinted `skip` closes its source

A `skip(n)` or `take(n)` that generic code asks the hint of is made
`$skipping` or `$taking` of its source, whose hint is std's (#1236). Its
source wasn't closed to drop what it holds, as the same adapter made
knowing its back is. Elsewhere in the crate, `ds().into_iter().skip(1)
.last()` of items whose drop panics then stopped dropping at the panic,
where `IntoIter`'s drop goes on. The two had the same type, so the one
made for its hint was made so everywhere. The source is closed now, as the
other's is. `test/corpus/skip_hinted_closes_its_source.rs` holds it.

## Amended (2026-11-10, #1240): what generic code makes and gives on

Generic code that makes an adapter of its own type parameter's iterator
and gives it to code that asks its hint, `hint(it.take(2).map(f))` of
`fn first_two<I: Iterator<Item = u32>>(it: I)`, made it JS's own,
`Iterator.from(it).take(2).map(f)`, `(0, None)`, where std's is
`(2, Some(2))`. The callers' instances' carrier (`instances.rs`) gave up
where a caller gives a type it makes of its own parameters,
`Map<Take<I>, F>`. It now follows such a type through that caller's own
callers, and keeps each one with its maker. `hint_asked.rs` marks what the
maker makes of its parameters, `Map<Take<I>, F>` and `Take<I>`, as made
knowing its hint in that function alone (`Hinted::knowing_in`,
`counted_in`): `Take<I>` is the same type in every generic function whose
parameter is `I`, and another's isn't asked its hint. It marks them where
one of its instances steps an array's items, or where callers it can't
see, a library's consumers, may give one. A type parameter itself is never
marked. The maker makes `$mapEnds` and `$taking` of `$given(it, IIterator)`
or `$closing(it, drop)`, as a JS iterator of what it's given, whose hint
`$given` and `$Steps` know, its source closed as #1244's is.
`as_double_ended` leaves as it is a type of generic code that its own
bounds don't make double-ended, `I` or `Take<I>` of an `I: Iterator`:
nothing can step it from its back, here or where it's given
(`may_hold_by_bounds`). Held by
`adapter_made_in_generic_code_asked_its_hint.rs` (a map, a take and a
skip of a slice's, a range's and a `Vec`'s iterator; a range given with its
dictionary; another generic function's `take` of items whose drop panics,
still dropping the rest), the crate pair
`library_adapter_made_in_generic_code_asked_its_hint`, and seven
mutations.


## Amended (2026-11-11, #1357): a std source an adapter holds hints as std's

`repeat_n(2u32, 4).peekable().size_hint()` was refused, "`size_hint()` of
a `Peekable<RepeatN<u32>>`, from its MIR", and so were a `fuse`, `zip`,
`chain`, `skip`, `map` or `enumerate` of it under a `peekable()`, and
`repeat(x).take(n)`'s, clippy's `manual_repeat_n`'s own form. A
`repeat_n(x, n)` alone was std's `(n, Some(n))`, but `held_hint_known`,
the one answer of what a held iterator's hint is, knew no std source.
`source_hint_kept` (`recognition/adapter_types.rs`) says which std
sources' runtime forms give std's own hint: `repeat`'s `$repeating` and
`repeat_with`'s `$repeatingWith`, `(usize::MAX, None)`; `repeat_n`'s
`$repeatingN`, what it has left both ways; `successors`' `$successors`,
`(1, None)` while it holds one, else `(0, Some(0))`; `from_fn`'s
`$fromFn`, a `$Steps`, std's `(0, None)`. `once`'s, an array made in
place, stays refused under a `peekable()`.

A wrong answer found on the way (P0): an explicit adapter, a `Take` a
struct's field holds, is known by `explicit_adapter` whatever it holds, and
`$successors` and `$repeatingWith` knew no hint, so `$takeHint` of them
was `(0, Some(n))` where std's is `(1, Some(n))` and `(n, Some(n))`, in
both builds. Each form now carries std's hint, as `$repeating`'s does.

Held by `repeat_n_hint_held.rs` (each adapter under a `peekable()`, a
peek, a `Vec`'s collect's capacity, `from_fn`'s, `successors`' and
`repeat_with(f).take(n)`'s) and `source_hint_where_an_adapter_holds_it.rs`
(explicit `Take`s of `successors`, of one made of `None` and of
`repeat_with`, stepped, chained and collected, and a held `Peekable` of
`successors`), against native Rust in both builds, and the mutations
`source-hint-unkept`, `source-hint-of-successors-unkept`,
`successors-hint-unknown` and `repeating-with-hint-unknown`.
`held-size-hint-refused` survived the whole corpus once a `Take` of a
`repeat` was a source's: `held_explicit_adapters_hint.rs` holds it again
with a `Take` of a `once`, which only the explicit adapter's rule knows,
and with a held `Take` of a type parameter's. Neither case prints
`repeat(x)`'s hint alone: its `usize::MAX` is wasm32's, rust-js's target,
where the host's native run's is wider. `repeat_n_hint_held.rs`'s
`generic-instances` rewrite, a `capacity()` after generic code's collect,
is #1263's.

## Amendment (2026-11-11, #1375): `once` and `once_with` hint as std's

Case ([0262](0262-when-rust-and-js-disagree.md)): none for results;
refusals lifted. `size_hint()` of `iter::once(x)` and `iter::once_with(f)`
was refused, "from its MIR". So was a `peekable()` of either, and a `Vec`
that a crate reading capacities grows by a chain of `once_with` with what
isn't `TrustedLen`. Std's hint is exact for both: `(1, Some(1))` until
stepped, then `(0, Some(0))`.

`once(x)` is `[x]`, an array of items, as an `Option`'s `into_iter()` is.
So it joins `iterates_items`, the one answer of which std iterators' JS
values are their items' arrays, and is made `$iter([x])` where its hint is
asked. Where nothing asks it, it stays `[x]`: `[3].map(f).reduce(..)`.

`once_with(f)` is `$onceWith`. It now carries std's hint by whether it
still holds `f`, which it still calls only when first stepped. It joins
`source_hint_kept` (#1357), the sources whose forms give std's hint.

Held by corpus case `once_and_once_with_hint.rs`, against native Rust in
both builds: each hint before and after a step, a `peekable()` of each,
one peeked, and a chain's and a zip's collect with its capacity.
Mutations `once-no-array-of-items`, `once-with-hint-unkept` and
`once-with-hint-after-stepped` are new; `once-with-calls-f-where-made` and
`once-with-calls-f-each-step` are refitted to the new form.

## Amendment: a held `skip_while`, `take_while`, `map_while`, `scan` and a dropping `filter`; adapters made in place inside another (2026-11-12, #1169, #1427)

Case ([0262](0262-when-rust-and-js-disagree.md)): none for results. One
refusal lifted, and two wrong answers made refusals.

**Lifted (#1169).** In the build that makes adapters in place, a held
`skip_while`, `take_while`, `map_while` or `scan` is always the runtime's
`$Steps` (`$lazySkipWhile` and the others), and a `filter` of items with a
destructor is `$filterDropping`'s `$Steps` filter. Each has std's hint,
`(0, upper)` of its source's, and `take_while`'s is `(0, Some(0))` once it
has stopped. Their `size_hint()` was refused. `asks_its_sources_hint` now
names the four, beside `decode_utf16`'s and `cycle()`'s, and
`adapted_hint_known` names the dropping filter, each asking its source's.

**Wrong answers, now refused (#1427).** `adapted_hint_known` judged an
adapter's sources by `held_hint_known`. That answers a `skip`, a `take`, a
`map` or an `enumerate` as std's struct, which knows its hint. But a source
given to an adapter the recognizers make is made in place too (ADR 0871),
JS's own over an array's items, `xs.values().drop(1).filter(f)`, which
keeps no hint. So a held `skip(1).filter(..)` hinted `(0, None)` where Rust
gives `(0, Some(2))`, and a held `map(..).cycle().take(2)` hinted
`(0, Some(2))` where Rust gives `(2, Some(2))`. Such a source is now judged
by its own sources (`made_in_place_of`), but not a `rev()`, whose in-place
`$rev(..)` or `$iter(xs.toReversed())` keeps its source's hint. The
refusal says so and names #1427, which tracks making those shapes know
their hint.

Held by corpus case `held_lazy_adapters_hint.rs` against native Rust in both
builds, `size_hint_of_adapters.rs`'s held `rev().filter(..)`, and two
diagnostics rows; guide limitations.md.

## Amendment (2026-11-12, #1333): a concrete adapter made knowing its hint by the function that makes it

Case ([0262](0262-when-rust-and-js-disagree.md)): none; readability. A
`take(n)`, `skip(n)` or `step_by(n)` of a type of no type parameter that
generic code asks the hint of (#1236) was marked by its type alone
(`Hinted::types`): one function asking made every one of that type in the
crate `$taking`, a `v.iter().take(2).sum()` that no one asks too. Now
`hint_asked.rs`, the one owner of the question, keys it by each function
that makes one whose value may leave it, `may_leave`: the type held in
what the function returns, takes or captures (fields of the crate's
structs and revealed opaque types included, an unrevealed one taken to),
in an argument of a call of what isn't std's, in what a closure it makes
captures, or unsized into a `dyn`. Values of one type move between
functions (#721), and each way out is one of those. Lowering reads the
fact, `counted_in` by its function (`made_knowing_its_hint`); `types` is
the analysis's scratch, empty once the crate is read. One made and
stepped where it's made is JS's own adapter: `v.slice(0, 2).reduce(..)`.

Returned or captured, a `take` is held, std's compiled struct (ADR 0871),
which knows its hint whether it's marked or not, so those two of
`may_leave`'s questions change no output today: they keep the analysis
true of itself should what's held change (#1458), and have no mutation.

Held by corpus case `takes_made_knowing_where_they_leave.rs` (consumed in
place, given back, given to a function of the crate's, captured by a
closure that asks, given to the asker where made) against native Rust in
both builds; no other snapshot changed. Mutations
`take-made-knowing-crate-wide` (snapshots), `take-left-unmarked`,
`take-passed-unmarked`.
