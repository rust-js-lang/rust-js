# 1165. A bound nothing reads is given no dictionary

Status: Accepted (2026-11-15, the lead's approval of #1915's first step,
each read route a row). Step 1 of #1915: the bounds of traits whose
dictionaries no other analysis answers for. Copy's
([0289](0289-copy-bounds.md)), Clone's ([0914](0914-a-librarys-generic-function-is-its-programs.md)),
the iterators' ([0411](0411-an-iterator-given-as-it-is-has-its-dictionary.md)),
`IntoIterator`'s, `Eq`'s and `Index`'s stay with their owners until each
migrates, deleting its own code (`answered_elsewhere`, held by
test/architecture-generic-facts.test.ts); so does a trait that extends
one, `trait Copied: Copy`, `Ord`, whose dictionary carries the owner's,
`TCopied.Copy()`.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. A dictionary no
code reads changes nothing a program can see; leaving it out makes the
function's JS the one a JS developer writes, `createDetails(given,
source, cancelable)`, upstream's three arguments.

## Context

A generic function's JS takes a dictionary of each bound of its type
parameters that has one, `eachShown(items, TDebug)`, whether its body
reads it or not. Upstream's createDetails (ADR 1164) is
`createDetails<G, C extends boolean = true>(given, source, cancelable)`;
the port's `create_details<G, C: Cancelable>` was given a fourth,
`TCancelable`, its declaration's types alone using `C`. An unread
dictionary is also a caller's work: each call builds one for nothing.

## Decision

**A bound `P: Tr` of a crate's function, free or an inherent impl's, is
given its dictionary only where the function reads it**:
`src/lower/analysis/read_bounds.rs`'s `unread_bounds`, a fixpoint over
the crate's bodies, says which are unread, and `traits.rs`'s `bound_of`
gives those none (`given_none`), at the function and at each of its
callers. A trait's method or a trait impl's keeps every bound its trait
declares, as a dictionary's maker gives them.

The function reads `P: Tr` where its MIR, or a closure's or an inline
constant's it makes, does any of these (each a row of
test/corpus/bounds_given_a_dictionary_where_read.rs):

| Route | Example | Reads |
|---|---|---|
| A method of `Tr` or of a trait it extends, of a `Self` that is `P` | `format!("{item:?}")` in a closure; `value.speak()` of `T: Loud`, `trait Loud: Speak` | `P: Tr`, or each bound that carries it |
| A default method its impl doesn't override | `value.shout()` | `P: Named` |
| An associated function without `self` | `T::made()` | `P: Named` |
| An associated constant or type | `T::NAME`, `<P as Tr>::Item` by value | `P: Tr` |
| A callee given `P` that takes a dictionary of it | `relay` calls `each_shown(items)` | as the callee's bound is read, by this fixpoint or a library's contract |
| A trait's method of a type holding `P`, `&T == &T` | `same_shown` | what the impl rustc selects asks, `A: PartialEq<B>` of `impl PartialEq<&B> for &A` |
| A `dyn` made of what holds `P` | `value as &dyn Speak` | each of `P`'s bounds |
| A drop of a type built of `P` | `Guard(value)` dropped, `impl<T: Speak> Drop for Guard<T>` | each of `P`'s bounds; a bare `P`'s is its `dropT` |
| A library's function that isn't a binding, given `P` | `TypeId::of::<T>()` | each of `P`'s bounds |
| A trait's other argument where no dictionary of `P`'s carries the trait | `1.into()` to `P`, a blanket impl's | each of `P`'s bounds |
| A body without MIR | | each of its function's bounds |

A type built of `P` given where a bound asks of it, `Vec<P>: Debug`,
reads each of `P`'s bounds where no impl rustc selects says which: an
impl may ask any. A projection behind a reference or a pointer, `&'static
C::Details<G>`, reads nothing: its JS is the value, which asks none of
the dictionary.

**A library's contract lists each export's unread bounds**,
`TypeParam.unread`, keyed by trait (library contract VERSION 42); its
consumer gives none of them. A library of an earlier ABI lists none:
each is given, as before.

## Consequences

- Six corpus snapshots lost a dictionary parameter nothing read,
  `fresh(TZero, TBump)`, `get(node)`, `keep(w)`; none changed what it
  prints. A namespace's method of a bound more than its callee's that it
  never reads is now the function it calls (test/namespaces.test.ts).
- A missed read is a dictionary not given where the body reads it, so
  the JS throws: each route's mutation
  (scripts/mutations/lower/analysis/read_bounds.ts) is caught by the
  case running. An over-read is caught by its snapshot.
- #1915 stays open for the later steps: Copy, Clone, the iterators,
  `IntoIterator`, `Eq` and `Index` migrate here one at a time, each
  deleting its old owner's code.
