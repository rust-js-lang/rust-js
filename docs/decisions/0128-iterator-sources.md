# 0128. std's iterator sources: `once` is an array, and `repeat` a JS iterator

Status: Accepted. Extends [0036](0036-iterators-and-sorting.md) and [0055](0055-iterator.md).

Case: C, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

std makes iterators out of nothing: `iter::once(x)`, `iter::empty()`,
`iter::repeat(x)`, `iter::repeat_with(f)`, `iter::successors(first, f)` and
`iter::from_fn(f)`, and an `Option`'s `iter()` and `into_iter()`. All were
errors, and so was `unzip()`.

`once` and `empty` have all their items already, as a slice's iterator does
(ADR 0036). `repeat`, `repeat_with`, `successors` and `from_fn` may never
end, as one of the crate's own may not (ADR 0055): as an array, `repeat(0)`
would never be made.

## Decision

| Rust | JS |
|---|---|
| `iter::once(x)` | `[x]` |
| `iter::empty()` | `[]` |
| `opt.iter()`, `opt.into_iter()`, `for x in &opt` | `opt == null ? [] : [opt]` |
| `iter::repeat(x)` | `$repeating(x)`, a JS generator, or `$repeating(x, clone)` |
| `iter::repeat_with(f)` | `$repeatingWith(f)` |
| `iter::successors(Some(1), f)` | `$successors(1, f)` |
| `iter::from_fn(f)` | `$fromFn(f)` |
| `xs.into_iter().chain(repeat(0))` | `$lazyChain(xs, $repeating(0))` |
| `s.chars().zip(repeat(7))` | `$lazyZip(Array.from(s), $repeating(7))` |
| `successors(..).take_while(p)` | `$lazyTakeWhile($successors(..), p)` |
| `pairs.into_iter().unzip()` | `$unzip(pairs)` |

- **The endless four are JS iterators**, and the adapters on them are the
  JS iterator helpers, which take only as far as they're used (ADR 0055).
- **`repeat` clones its value for each item**, as Rust's does, when the
  value is one a clone copies (ADR 0052): `repeat(vec![0])` gives a new
  array each time.
- **`successors` finds the next item before it gives this one**, as
  Rust's does, so its closure runs as often as Rust's.
- **`from_fn` calls `f` for each `next()`**, after a `None` too, as Rust's
  does, so it isn't a generator, which would end at the first `None`.
- **Closures giving a generic `T`'s `Option`** box a `Some` that looks like
  `None` (ADR 0051): `successors` and `from_fn` unbox it.
- **`chain` and `zip` are lazy when either side is**: an array's `concat`
  would add a JS iterator as one item, and `$zip` wants lengths. What's
  chained or zipped on is a JS iterator when it's one of the crate's own,
  which it wasn't: `xs.iter().chain(counter)` was its struct, not its items.
- **`take_while` and `skip_while` of a JS iterator** are generators, no
  longer errors.
- **A JS iterator in a local that `next()` steps through** is itself, not a
  `$iter` of it (ADR 0071), which wanted an array: `let mut r = repeat(4);
  r.next()` was `None`.
- **`unzip` into two `Vec`s** is two arrays; into anything else it's an
  error.
- **A `&mut Option`'s items** are places, and iterating over one is still
  an error.

## Why

- **It's the JS a person writes**: `[x]` for one item, and a generator for
  an endless iterator.
- **It's exact**: the generators take as many items, and call their
  closures as often and in the same order, as Rust's.

## Consequences

- The sources, and `chain`, `zip`, `take_while` and `skip_while` on JS
  iterators, compile, compared with native Rust by the `iterator_sources`
  corpus case.
- `once_with`, `repeat_n` and `cycle` are still errors.

## Amendment: `iter::Empty` is an iterator over an array

`iter::empty()` is `[]`, and from MIR, as an iterator over an array is,
`$iter([])` (ADR 0364): so `iter::Empty` is one of `array_source`'s, which
own their items, here none, and what's true of one is true of it, its
clone what it has left (ADR 0181). A constant of one, rustc's
`consts/std/iter.rs` and the corpus's `std_struct_const`, was an error, as
its clone wasn't known, and runs: its uses read `NOTHING.items.slice(..)`,
what the constant has left, wordier than `[]`, as a constant iterator,
which is rare, is held whole.

## Amendment: `unzip` into a map or a set

`unzip` into what isn't a `Vec` was refused (#165). A side that's a map
or a set is made empty and extended by its items, `$extendMap(new
Set(), unzipped[1], true)`, as std's `unzip` extends each side's
`Default`. Held by `test/corpus/unzip_into_maps.rs`.

## Amendment: `once_with` (#1373)

`std::iter::once_with(f)` was refused. It's a JS iterator, `$onceWith(f)`, a
generator that yields `f()`, as `repeat_with`'s is: `f` runs when it's first
stepped, never where it's made, and an iterator never stepped never calls it.
Its type is `TrustedLen`, so a `Vec` collected from it has room for exactly its
one item, as native Rust's; its `size_hint()`, and a `Vec` grown by a chain or
zip of it with what isn't `TrustedLen` in a crate that reads capacities, refuse
as `repeat_with`'s do, whose hint JS's generators have none of (`Once`'s is
refused too). Its `next_back` is refused as any generator's is. Case none of
[0262](0262-when-rust-and-js-disagree.md). Held by
`test/corpus/iter_once_with.rs`; mutations `once-with-calls-f-where-made`,
`once-with-calls-f-each-step` (`runtime/once_with.ts`) and `once-with-unanswered`
(`lower/recognition/calls.ts`).

## Amendment: an `iter::Empty` constant (#1420)

A consumer compiles some of a library's bodies itself, from their optimized
MIR (ADR 0914), quickcheck's `Arbitrary::shrink` and `empty_shrinker` among
them. There, inlining leaves `iter::empty()`'s value as a constant of no size,
`const Empty::<A>(PhantomData)`, which was refused, "this constant, from its
MIR", so itertools' 142 quickcheck tests never ran. `StdZeroSized`
(`representation/consts.rs`) is the one answer to what each of std's types of
no size is in JS where that isn't its struct: `iter::Empty`'s `[]`,
`io::Empty`'s, `PhantomData`'s and `BuildHasherDefault`'s `undefined`. It
answers `iter::empty()`'s and `io::empty()`'s calls, `Default` (which makes
`Default` of an `iter::Empty` or an `io::Empty` supported where it was refused)
and a constant of one, which a body holds as it holds the call's value
(`std_value_as`): `$iter([])`. `zero_sized_js`, where no body holds it, still
writes none, as `const_js` writes no std struct whose JS isn't its fields.
No program's JS changes. Case none of
[0262](0262-when-rust-and-js-disagree.md). Held by
`test/crates/pairs/empty_of_a_librarys_default_compiled_by_its_consumer`, against
native Rust; mutations `std-zero-sized-iter-empty-unknown`,
`std-zero-sized-io-empty-unknown`, `build-hasher-default-refused`
(`representation/consts.ts`) and `std-zero-sized-not-held` (`mir/operands.ts`).
