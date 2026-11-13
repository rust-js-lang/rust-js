# 0445. Generic code extends and collects into what it's given

Status: Accepted. Extends [0062](0062-combinators.md),
[0098](0098-destructors.md), [0106](0106-generic-traits.md),
[0160](0160-user-collections.md) and [0180](0180-generic-writers.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): the same program,
each value where Rust puts it.

## Context

Shared models are written against traits: either's `Either` extends and
collects into whatever its sides are, `impl<L: Extend<A>, R: Extend<A>>
Extend<A> for Either<L, R>`, and itertools builds on either. Both stopped
at rust-js's first refusal, `calling Extend::extend`, and behind it were
more, each of a common idiom:

- `s.extend(chars)` of a `String`, refused even where the type is known.
- `c.extend(items)` of a `C: Extend<A>`, and `collect()` into a `B:
  FromIterator<A>`, `B::from_iter(items)`: generic code had no dictionary of
  either trait.
- `nth`, `find` and `partition` of items with a destructor, refused because
  std drops what they pass over, which rust-js didn't.

## Decision

**`Extend` and `FromIterator` are given dictionaries, as `AddAssign` is
([0396](0396-a-compound-assignment-in-generic-code.md)): generic code calls `extend` and `from_iter` through them, and
a known collection's call is written in place, as before.**

```rust
fn fill<C: Extend<u32>>(c: &mut C, n: u32) { c.extend(0..n); }
fn gather<B: FromIterator<u32>>(n: u32) -> B { (1..=n).map(|x| x * 2).collect() }
```

```js
function fill(c, n, CExtendU32) {
  CExtendU32.extend(c, $lazyRange(0, n));
}
function gather(n, BFromIteratorU32) {
  return BFromIteratorU32.from_iter($lazyRange(1, n + 1).map((x) => Math.imul(x, 2) >>> 0));
}
gather(3, { from_iter: (items) => Array.from(items) });
```

- **A std collection's dictionary is what its known call is**, of the
  items given, any iterable: a `Vec`'s `$extend`, a map's or a set's
  `$extendMap`, a `String`'s text added; `from_iter` what `collect` into it
  makes. Its `extend` takes the box generic code's `&mut` is (ADR 0180).
  One function answers what a std collection's `extend` is,
  `Recognition::std_extend`, which the known call and the dictionary ask.
- **The crate's own impl is its dictionary**, written only where generic
  code reads it (`read_only_generically`), as an operator's is.
- **A range or an iterator of the crate's given a std method's own generic
  `IntoIterator`**, `c.extend(0..n)` through a dictionary, is a JS iterator,
  as one given a function of the crate's is (ADR 0061).
- **`s.extend(items)` of a `String` is a `push_str`** of their text,
  `s += ["y", "z"].join("")`; a range's or an iterator's, `Array.from(..)`
  of it, joined.
- **What a method passes over of items with a destructor it drops**, as
  std's does (ADR 0098): `nth(n)` each it skips, `$nthDropping`, and
  `find(test)` each it rejects, `$findDropping`; `partition` moves each into
  one of its two, as `extend` does, and drops none.

## Why

- **It's Rust's**: generic code over `Extend` and `FromIterator` is how a
  shared model, and the crates it uses, are written; `s.extend(chars)` is
  how Rust builds text.
- **One form for each**: a known collection's call is written in place, a
  generic one's through its dictionary, as every operational trait is.
- **It's tested**: corpus cases extend and collect generically into std's
  collections, text, a `Result` and the crate's own, and drop what `nth`
  and `find` pass over, each against native Rust; a mutation for each step
  is caught.

## Consequences

- either compiles past them, and stops at `rfind`, `nth_back` and `rfold`,
  a double-ended iterator's, which rust-js doesn't lower yet.
- Giving a generic function an iterator of items with a destructor where
  it takes no drop for it is still refused (ADR 0190).

## Amendment: a `String`'s `extend` pushes each piece as it's stepped (2026-11-10, #1127)

A `String`'s `extend` joined its items' text, then appended it once: `s +=
Array.from(items).join("")`, or `$foldToArray` since #1126. Std's pushes
each `char` or string as `for_each` gives it, so a panic part-way, caught,
leaves what came before it pushed. `text.extend(['a', 'b', 'c']...map(|c|
if c == 'c' { panic!() } else { c }))` under `catch_unwind` left `">ab"`
natively and `">"` as JS, of a plain iterator as of the crate's; and so did
generic code's, through `Extend`'s dictionary.

**Of an iterator whose steps may panic or that holds what drops, each piece
is given a function that appends it as it's stepped, `$extendText(it,
(piece) => { s += piece; })`,** which steps it as `$foldToArray` does: by
its own fold where it has one, what's left of it closed as a panic unwinds,
then at its end (#1126). Generic code's dictionary is given any iterable,
so it does so always. Of another, a range's or an array's, which can't
throw, its text joined and appended once, as before (`joined_text`), as
nothing tells the two apart.

Held by `test/corpus/iterator_consumed_by_extend_unzip_partition_eq.rs`,
against native Rust in both builds: the crate's iterator with a `Drop`, a
plain one and generic code's, each caught part-way, and a range of `char`s.
Four corpus cases' JS changed to `$extendText`, none in what it prints.
Mutations: new `extend-text-pieces-lost`, `extend-text-iterable-unstepped`
(`runtime/fold_of.ts`) and `generic-text-extend-joined-nothing`
(`lower/traits/dictionaries.ts`); `text-extend-unclosed`,
`string-extend-unjoined`, `string-extend-unreserved`, `mir-push-str-lost`,
`extended-cows-as-objects`, `joined-text-of-iterable-unarrayed` and
`joined-text-own-items` refitted.

## Amendment: a collection's `from_iter` is the collect generic code's own is (2026-11-10, #1239)

Generic code's `collect()` into a `B: FromIterator<A>` calls the
dictionary its caller gives, `B.fromIter(it, IInPlace)`. A std
collection's entry was its own lowering, `$foldToArray(items)` then made
into the collection, where the same collect written into a `Vec` in
generic code, `it.collect::<Vec<I::Item>>()`, grows as std's
`SpecFromIterNested` does: `$roomForHint(it, size, ..)`, which asks the
hint as it grows. Two lowerings of one rule, and the dictionary's asked no
hint, so `collect::<B>()` of `B = Vec<u32>` printed none of a hint that
prints, among 54 corpus cases' `generic-instances` rewrites (#1237).

The entry is now that one lowering (`from_iter_entry`): the collect of a
type parameter `I: Iterator<Item = A>`, bound so in the entry alone, its
facts the entry's parameters after the items, as `from_iter`'s declaration
carries them (`std_method_facts`, #794), `IInPlace`, which generic code's
call already gives: the call keeps its shape. What's left of what it's
given is closed where collecting it panics, as std drops the iterator its
collect holds (#1141), which the `$foldToArray` it replaces did: the
runtime's `$collecting(items, () => ..)`, of `$unwinds`, one helper for
every entry, the collect itself the compiler's; the type parameter says
nothing of what it holds, so it's always closed then. An entry reads like
`fromIter: (items, IInPlace) => $collecting(items, () =>
Array.from($roomForHint(items, 4, false)))`, and one a body gives twice is
declared once, `vecFromIterator` (ADR 0979, amended). A `Vec` so collected is
`$untracked` as generic code's own is, std perhaps collecting it in place
(#1180), so its `capacity()` is refused at run time, not a number std's
isn't. Held by `collect_into_a_type_parameter_is_stds.rs` (a `Vec`, a
filter's, a `VecDeque`, a `String`, a `HashSet`),
`consumer_closes_what_it_steps_on_a_panic.rs`, and the mutations
`from-iter-entry-folds-to-an-array`, `collecting-left-unclosed` and
`from-iter-dictionary-unclosed`, refitted; 39 of #1239's 52 cases' rows run as
Rust does.

## Amendment: a std collection's `extend` drops the iterator it's given (2026-11-13, #1511)

Generic code's `c.extend(it)` of a `C: Extend<A>` calls its dictionary's
`extend(collection, items, ..)`, given after its items the drops of `T`
and of `T::IntoIter` the trait's method declares. std's dictionary was
`{ extend(collection, items) { $extend(collection.value, items); } }`. It
ignored those drops, so the crate's iterator with a `Drop` was never
dropped, where std's `extend` drops it at its end (`drop Up 3`). A list
took the `Vec`'s dictionary and stepped what it was given by `next`, never
reaching the crate's own `fold` that std's `for_each` calls, and a
B-tree's, likewise. And a crate's own `Extend`, given the JS iterator its
caller made of the crate's, was given the iterator type's drop of that JS
iterator, `drop Up undefined`. Results change, so Rust's behavior is kept
(ADR 0262).

**std's dictionary steps what it's given as std's `extend` steps it, then
drops it.** It holds the iterator, `const it = $intoIter(items)`. A `Vec`'s
and a deque's step it by `next`. A list's, a hash table's and a B-tree's
step it by `fold`, `$folded(it)`, as `extends_by_fold` says. Then it drops it,
`dropIter?.(it)`, at its end and as a step's panic unwinds (`$unwinds`),
except where the crate's own `fold` took it (`it.ownFold`), which drops it
itself. A trait's generic method called through a dictionary is given, for
a value its caller made a JS iterator of, the drop of what that was made of,
its `of` (`held_drop_function` of `iterated_slot`, as a direct call is).
A dictionary's function that declares a `const` of its own is still
declared once at the top where it's written twice (`reads_only_at_top`).

Corpus case `extend_through_a_dictionary_drops_its_iterator` runs against
native Rust in both builds. It covers a `Vec`, a `VecDeque`, a
`LinkedList`, a `HashSet`, a `BTreeSet` and a crate's own `Extend`, each
given an iterator with its own `fold` and one without, and a step that
panics, caught. Eleven snapshots change, each a std `Extend` dictionary.
Mutations `extend-dictionary-iterator-undropped`,
`extend-dictionary-not-by-fold`, `extend-dictionary-own-fold-dropped-again`,
`dictionary-call-drop-of-what-iterated-made` and
`arrow-body-declarations-unbound` are new;
`dictionary-call-own-drops-missing` is refitted.

## Amendment: the entry's collect closes what it's given itself (2026-11-13)

Case: A ([0262](0262-when-rust-and-js-disagree.md)): what's printed is
native's with or without the wrapper.

A `fromIter` entry ran its collect inside `$collecting(items, () => ..)`,
which closed what it's given where collecting it panics. Each of the
runtime's collects does that already: `$foldToArray` and
`$collectDropping` close what's left as a step unwinds, and `$roomForHint`
closes it where asking room panics before any step, a `capacity overflow`.
The nightly's survivor `from-iter-dictionary-unclosed`, which took the
wrapper out, showed that no case drops differently without it. So the
wrapper is gone, and with it the runtime's `$collecting`: one close, the
collect's own. An entry reads `fromIter: (items, ITrustedLen, IInPlace) =>
new Set($reserveHint($folded(items), 4, 4))`. The corpus case
`generic_collect_closes_what_it_steps_on_a_panic.rs` collects into every
std collection generic code can (a `Vec`, a `VecDeque`, a `LinkedList`, a
`BinaryHeap`, both sets and both maps, a `String`, a `Box<[T]>`, an
`Option` and a `Result` of a `Vec`), of a step that panics part-way and,
for five of them, of a hint that overflows their capacity. It runs against
native in both builds, and so do the 13 cases whose JS lost the wrapper.
The mutations `from-iter-dictionary-unclosed` and
`collecting-left-unclosed` go with the code they mutated.
