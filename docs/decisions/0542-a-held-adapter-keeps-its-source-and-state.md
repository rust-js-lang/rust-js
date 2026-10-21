# 0542. An adapter a struct holds keeps its source and its state

Status: Accepted. Extends [0170](0170-size-hint.md), [0411](0411-an-iterator-given-as-it-is-has-its-dictionary.md),
[0487](0487-an-iterator-given-as-it-is-has-its-extensions-dictionaries.md) and [0528](0528-fuse-and-enumerate-from-either-end.md);
accepts the first part of the [iterators draft](draft/cloned-and-buffering-iterators.md)'s
core, its analysis choosing an explicit adapter before lowering.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's asked of an
adapter after it's made, its items from either end, its size hint and its
`Debug`, is Rust's.

## Context

itertools' `Positions` holds an `Enumerate<I>` and asks it, after it's
made, for its size hint, its back where its impl's bounds give `I`'s, and
its `Debug`, which shows its state, `Enumerate { iter: IntoIter([4, 5]),
count: 2 }`. Generic code's `enumerate()` was JS's `map((x, i) => [i,
x])`, which keeps neither its source nor its count: each was refused. The
draft asks one analysis to choose, before lowering, where an adapter must
keep its state, and native JS adapters elsewhere.

## Decision

**A kind of std adapter the crate's types hold is explicit wherever the
crate makes one: a JS iterator that keeps what it steps, `iter`, and its
state.** One kind is explicit everywhere in a crate, so no value of it is
converted midway.

- `analysis::held_adapters` is the one owner of which kinds: those a field
  of the crate's structs, enums or unions holds, through whatever holds
  them. `held_adapter(ty)` is the one answer the lowering asks.
- **`Enumerate` is the first kind**: `$enumerate(iter)` keeps `iter` and
  `count`, gives its own size hint, `hint`, its source's, and its back and
  length as its source's, asked as they're asked. It replaces
  `$enumerateEnds`, one helper. A held one's source is made as capable as
  its type: a range knowing its back, and an exclusive one its length.
- **A source given as it is (ADR 0411) gets its back and length where
  they're asked**: a use whose bounds give `I: DoubleEndedIterator +
  ExactSizeIterator` installs its dictionaries' on the `$iterator` the
  adapter steps, `$knowing(it, IDoubleEndedIterator.next_back,
  IExactSizeIterator.len)`, as the construction had only `I: Iterator`'s.
  `stepped_params` asks rustc what an adapter's impl asks of what it
  holds, `Enumerate<I>`'s back `I`'s length too, through a `&mut`.
  `held_knows_its_back` answers which held ones step from their back as
  they are.
- `by_ref()` of a held adapter is itself, which closing doesn't end, as a
  `$iter`'s is.
- **Its size hint is its own** (`$sizeHint` asks `hint`), and a
  `$iterator`'s that of the JS iterator it steps; a range given as it is
  still tells none, ADR 0170's difference.
- **Its `Debug` is std's**, `Enumerate { iter: .., count: n }`, of what
  its source steps, `$iteratorValue`, plain and pretty.

## Why

`held_enumerate` is `Positions`: forward, from its back by its impl's
bounds, folded from its back, its size hint and its `Debug` plain and
pretty, of a `Vec`'s iterator and of a range given as it is; a struct
holding an `Enumerate` of a range stepped from its back; an array's
`enumerate().rev()` in a crate whose types hold one; and a map of a `&mut`
to an iterator a struct holds stepped from its back, against native Rust.

## Consequences

- itertools' four `Positions` refusals and `WithPosition`'s size hint are
  gone; five remain, the draft's next kinds: `Peekable`, `Cycle`, a
  `PutBack`'s `Debug`, and a map folded from its back in a library.
- In a crate whose types hold an `Enumerate`, a chain's enumerate is
  explicit too, `$rev($enumerate($iter(indices)))`, where another crate's
  is JS's: the kind decides, not the place, so no value is converted.
- Not yet across crates: a library's held kinds aren't its consumers'.
  A consumer only steps what a library gives it, which an explicit
  adapter allows; one asking a library's adapter for its `Debug` or its
  back would need the library's kinds in its manifest.
- Mutations see no kind held, make one JS's, give a generic source no
  back, take an array's for unknowing, make a range unknowing its length,
  lend it, refuse its hint, keep no count, install nothing, read neither
  hint, and skip what an adapter's impl asks or what's behind a `&mut`.

## Amendment: a `Take`, a `Skip`, and one the crate clones

Case: A ([0262](0262-when-rust-and-js-disagree.md)): what's given, shown
and counted is Rust's; refusals are lifted.

**An adapter that keeps a count of its own, an `Enumerate`, a `Take` or a
`Skip` (`recognition::counted_adapter`), is explicit where the crate's
types hold its kind, as before, or where the crate clones one**
(`analysis::explicit_adapters`, which `explicit_adapter(ty)`, renamed from
`held_adapter`, answers):

- `$taking(iter, n, drop)` and `$skipping(iter, n, drop)` keep what they
  step and how many they have left to take or skip, `n`, as Rust's `Take`
  and `Skip` do: forward, from the back as Rust's `next_back` steps them,
  their length, size hint and `Debug`, `Take { iter: .., n: .. }`.
  `$skipping` replaces `$skipDropping`, the skip of items with a
  destructor, which kept its count hidden: one stateful skip.
- **A clone of one is one of a clone of what it steps, at its count**:
  `$enumerate(it, numbered.count)`, where `it` is a clone of
  `$iteratorValue(numbered.iter)`, the value its type has, made as
  capable as a held one's source is. Cloning one was refused.
- Closing one closes what it steps, `$closed(iter)`, as closing an
  adapter does (ADR 0465).

A count is state a JS iterator helper hides, `map`'s index or `take`'s,
so it's the adapter's own, where a clone, a `Debug` and a back can read
it: the state ADR 0525's made-again adapters don't have.
The readability measure's `$iter` share grows, 61.6 to 61.9 in ten
thousand lines, and its renamed one, 824.7 to 825.2: a clone of an
array's iterator is a `$iter` of what it has left (ADR 0530), and the
value a clone is made of is held, `value`, `value$1`, read twice, for
its iterator and its count, in the new corpus cases that clone them.
`test/corpus/cloned_counted_adapters.rs` clones each before and after it
steps, and holds a `Take` and a `Skip` in a struct, stepped from either
end, counted and shown, against native Rust. Mutations
`cloned-counted-not-explicit`, `counted-clone-from-start`,
`counted-length-unknown`, `held-steps-enumerate-only`,
`take-debug-as-enumerate`, `enumerate-clone-count-ignored`, and
`taking.ts`'s and `skipping.ts`'s.
