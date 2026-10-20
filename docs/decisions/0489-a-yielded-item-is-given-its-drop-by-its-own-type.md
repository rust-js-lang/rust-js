# 0489. A std associated type is given its drop by its own type

Status: Accepted. Amends [0421](0421-std-associated-type-drops-are-given.md),
extends [0178](0178-associated-type-drops.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's dropped, and
when, as std's; where no drop could be made, D: rust-js's error.

## Context

A function is given a drop for a std trait's associated type of its type
parameter, `dropIItem` of `<I as Iterator>::Item`, where it's given one for
`I` (ADR 0421). But an iterator with no destructor may give items with
one: `Gen`, which makes a `Loud` each step. `second(Gen(0))`, which skips
an item, wasn't given `dropIItem`, and leaked it, silently; and one given
as it is, with its `Iterator` dictionary (ADR 0411), looked for the drop
in the dictionary, `IIterator.$dropItem`, as a crate's trait's has it (ADR
0178), which std's `{ next }` never has.

And where nothing gave one, in a crate with a `Drop` impl, a value of the
associated type was refused, where a type parameter's drops nothing.

## Decision

**A std trait's associated type is given a drop where its caller's is a
type with one, whether its type parameter has one or not; or where it's
the caller's own type parameter, or own associated type, given one.** It's
found as its type, `dropIItem`, never in a dictionary, which a std trait's
has none of. **And one a caller could give a drop for, that none does,
drops nothing**, as a type parameter none's given for.

- **The analysis asks each call** what the callee's associated types are
  in the caller's terms, `drop_items` instantiated and normalized: a type
  with a destructor, given; the caller's `T`, or its `J::Item`, passed on.
- **A function knows which it could be given, `slots`**, keyed as its
  slots are, a copied default's in its impl's terms, `T::Item`; and one
  place, `slot_of`, finds a slot by its type, in those terms or, of a
  copied default's body, in its trait's, `Self::Item`, which `given_drop`
  and what a type drops both ask.
- **A trait impl's**, reached through a `dyn` its caller can't be seen to
  give, is given one for each of its parameters', as before (ADR 0421).
- **Another associated type's, which no slot gives**, `<T as
  Add>::Output`, is refused, as before.

```js
function second(iter, dropIItem) {
  const value = $nextSome(iter);
  if (value != null) {
    dropIItem?.($someValue(value));
  }
  return $nextSome(iter);
}
```

## Why

- **It's std's**: `yielded_item_drops` skips items with a destructor of an
  iterator without one, given as it is and as a JS iterator, directly, by a
  function whose `T` they are, by one whose `J::Item` they are, and through
  a `dyn`, and counts a range's where nothing has one, with native Rust.
- **A silent leak becomes the drop Rust runs.**

## Consequences

- **What compiles grows**: a value of `I::Item` where no caller's has a
  destructor, in a crate with one.
- **`<T as Add>::Output` and other associated types no slot gives** are
  still refused where a type may have a destructor.
