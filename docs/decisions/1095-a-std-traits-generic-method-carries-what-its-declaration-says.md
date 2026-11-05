# 1095. A std trait's generic method's dictionary carries what its declaration says

Status: Accepted (the user's decision, option (B), #794). Extends
[0466](0466-a-generic-maps-class-is-its-callers.md), whose dictionaries
carry a trait's generic methods' own type facts, from the crate's own
traits to std's; and [0445](0445-extend-and-collect-in-generic-code.md), a
collect into a type parameter by its dictionary's `from_iter`. Library ABI
17.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. What a call through a
dictionary gives is what the impl it reaches asks, so each collect drops
what a panic stops as Rust's does; where it would give `undefined`, the
drop order would differ (case: results change), which this prevents.

## Context

semver's `impl FromIterator<Comparator> for VersionReq` collects what it's
given, `Vec::from_iter(iter)`. Since c06ac4fdf (#672), a collect into a
`Vec` of items with a destructor asks the buffer std may collect its
iterator in, in place, `InPlace` of `I::IntoIter`, which decides what a
panic drops first. The impl's `from_iter::<I>` is std's trait's generic
method: its caller through `FromIterator`'s dictionary, a generic
`collect()`, gave none of its own type parameters' facts, as ADR 0466 has
only the crate's own traits' dictionaries carry them. So semver was
refused, "the buffer std collects an iterator in place in of `<T as
IntoIterator>::IntoIter`, which no caller gives", a regression of the
crate corpus (#794).

A std trait's method's facts can't be the union of what the impls a crate
sees ask, as a crate trait's are: a library calling `C::from_iter` through
a dictionary is compiled without another library's impl of it, and would
give less than that impl asks, a wrong drop order, not a refusal.

## Decision

**A std trait's required generic method's dictionary carries what its
declaration says, the same in every crate (`traits::std_method_facts`):
of each of its own type parameters bounded by `IntoIterator`, `InPlace`
of its `IntoIter`, and of one bounded by `Iterator`, its own** (std's
in-place rule, #672). `FromIterator::from_iter`'s, `Extend::extend`'s,
`Sum::sum`'s and `Product::product`'s.

- **One answer, `traits::foreign_facts`**: what a function compiled
  elsewhere asks, a std required method's declaration's facts or a
  library's manifest's, which the analysis relays to callers and a call
  gives (`facts_asked`, `evidence_slots`).
- **Only through a dictionary.** A call rustc resolves to std's own impl,
  `Vec::from_iter(it)`, asks what that impl's lowering asks (#672's
  `in_place_asks`), not what the trait carries.
- **A `collect()` into what the caller's own bound collects into**, `B:
  FromIterator<A>` or `Result<U, E>: FromIterator<..>`, calls its
  dictionary's `from_iter` with the iterator's facts and drops after the
  items (`own_evidence`), and asks them of its caller where they're its
  own: `tryCollect(it, ResultFromIteratorResult, IInPlace)`.
- **An impl's entry takes them**, after its own dictionaries, as a crate
  trait's method's are (ADR 0466): `reqFromIteratorD_fromIter(iter,
  IIntoIterInPlace, dropI, dropIIntoIter)`. One that asks a fact the
  declaration doesn't carry is refused, as before.
- **A provided method's std body, compiled with the crate, asks its own**
  (ADR 0561): `Iterator::zip`'s carries what its body asks, not this.
- **The crate's own `impl FromIterator<A> for X` is its own entry**, not a
  std collection's: `has_user_impl(trait, ty)` rebuilt `FromIterator`'s
  arguments from `ty` alone, missing `A`, so an entry collected
  `Array.from(items)` into it and gave its impl `undefined`; now
  `is_user_impl(tr)`, of the trait ref.

No list of std methods and their facts is written in a library's
manifest: each crate derives the same from std's declaration, and a
library function's own asks are its `calling.params[].facts`, from the
analysis. The ABI's bump keeps a library built before this from meeting a
consumer built after it.

## Why

- **Sound across libraries**: the facts are the declaration's, so a library
  compiled without an impl gives what that impl may ask.
  `test/libraries.test.ts`' "a std trait's generic method's dictionary
  carries the same facts in each library" builds that trap, a library's
  generic collect and another's impl, each without the other.
- **The idiomatic impl compiles**: `impl FromIterator<C> for X {
  Vec::from_iter(iter) }`, semver's, with std's drop order.

## Costs

- **An impl's entry that doesn't read a fact still takes it**, `{ fromIter:
  (arg0, TIntoIterInPlace) => bagFromIteratorU32_fromIter(arg0) }`: an
  entry that passes on just what it's given is the method only where its
  parameters are all it passes (`passed_on`), which doesn't check the
  method's own arity, so callers' evidence past them could reach its other
  parameters.
- **A panic partway through the in-place collect** drops what it took per
  the fact only where the collect's own drop does, which a generic
  `Vec::from_iter` doesn't yet (#800).

## Tests

`test/corpus/from_iter_impl_of_dropping.rs` (a direct collect, a generic
one through the dictionary, of a range's map and of a `Vec`'s iterator, and
one relaying its own `I`'s fact), `generic_extend`, `generic_accumulations`,
`nested_item_drops` and `generic_collect`, each against native Rust, as a
program and a library; `test/libraries.test.ts`' library pair; `bun
scripts/crate-corpus.ts --check semver`, which compiles. Mutations:
`std-method-facts-none`, `std-method-facts-of-provided`,
`std-method-facts-unrelayed`, `std-method-facts-of-resolved-calls`,
`bound-collect-unrelayed`, `user-from-iterator-as-collection`,
`dictionary-collect-without-own-evidence`.
