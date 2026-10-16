# 0421. A std trait's associated type's drop is its caller's to give

Status: Accepted. Extends [0098](0098-destructors.md), whose type
parameters' drops a caller gives, [0163](0163-trait-method-drops.md) and
[0178](0178-associated-type-drops.md), and amends
[0061](0061-generic-iterators.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a value is dropped
where Rust drops it, once.

## Context

Generic code drops values of a std trait's associated type of its type
parameters: an `I: IntoIterator`'s iterator, `I::IntoIter`, when it's done
with it, an `I: Iterator`'s item, `I::Item`, that it doesn't keep.
smallvec's `extend` does, of whatever it's given: another `SmallVec`'s
`Drain`, whose `Drop` puts back what it didn't take.

```rust
fn extend<I: IntoIterator<Item = A::Item>>(&mut self, iterable: I) {
    let mut iter = iterable.into_iter();
    ...
    for elem in iter { self.push(elem); }
}
```

A crate trait's associated type has its drop in its impl's dictionary
(ADR 0178); a std trait has no dictionary, and holding a value of one was
an error wherever a type may have a destructor, and a loop over one took
no destructor of its type parameter (ADR 0190). Their caller knows what
they are: a `vec::IntoIter<Loud>`, a `Drain`.

## Decision

**A generic function is given a drop for each std trait's associated type
of its type parameters, `dropIIntoIter`, as it's given one for a type
parameter, by its caller, of the type it is there.**

```js
function first(items, dropIIntoIter) {
  const it = Iterator.from(items);
  ...
  dropIIntoIter?.(it);
  return result;
}

first(loud, (intoIter) => { for (const item of intoIter) loudDrop_drop(item); });
```

- **Which it's given is its bounds'**, `traits::drop_slot`: after its type
  parameters', each associated type of a bound of a trait no dictionary
  carries, `<I as IntoIterator>::IntoIter`, `<I as Iterator>::Item`, its
  supertraits' too, in its bounds' order. Its callers, a dictionary's, and
  a library's consumers read the same bounds, so they agree, and a
  library's manifest lists them as it lists its type parameters' drops.
- **One a bound names isn't one**: `IntoIterator<Item = A::Item>`'s `Item`
  is `A::Item`, its impl's dictionary's.
- **It's given where its type parameter is** (ADR 0098), and taken only
  where it's used (ADR 0300); a trait's generic method's are its
  declaration's, which a call through a dictionary gives (ADR 0163).
- **An iterator of the crate's given as a JS iterator,
  `$iterator(value, next)`** (ADR 0061), **is dropped as `value`**, its
  `of`: what generic code holds of it is the JS iterator, of a type
  parameter's drop or its `IntoIter`'s, `iterators::iterated_slot`. A
  trait impl's method is given a JS iterator of one as a function is,
  `iterators::given_iterators`.

## Why

- **It's the program Rust runs**: `test/corpus/std_projection_drops.rs`
  drops what's left of a `Vec`'s iterator and of an iterator of the
  crate's with a `Drop`, given to a function, passed on to another, and to
  an `Extend` impl that leaves early, and an `I::Item` it doesn't keep,
  with native Rust; `test/crates.test.ts` a consumer's, in a library's
  generic function.
- **It's a type parameter's drop**: the same `?.()`, given the same way.

## Consequences

- A type parameter given no value with a destructor is given no drop for
  its associated types either: one with none whose iterator has one, a
  `Copy` type's, is an error still.
- An adapter of std's holding an item with a destructor, `take`'s of a
  `repeat`, is still an error, its drop JS's iterator's.
- Before, a crate's iterator given as a JS iterator was dropped as the JS
  iterator, which its `Drop` read as its own: a silent difference, now the
  value's.
