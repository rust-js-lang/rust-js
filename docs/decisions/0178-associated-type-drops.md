# 0178. An impl's dictionary has its associated types' drops

Status: Accepted. Extends [0098](0098-destructors.md),
[0106](0106-generic-traits.md) and [0163](0163-trait-method-drops.md).

Case: C, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

Generic code can hold a value of an associated type, whose type only the
impl knows:

```rust
pub struct DateTime<Tz: TimeZone> {
    datetime: NaiveDateTime,
    offset: Tz::Offset,
}
```

Dropping one runs the impl's type's destructor, if it has one. A type
parameter's drop is given by its caller (ADR 0098), but nothing gave one
for an associated type, so holding one was an error in any crate where a
type may have a destructor: one of its own, or any library's. That stopped
chrono, at 55 places.

## Decision

**An impl's dictionary has a drop for each associated type whose type has
a destructor, `$dropOffset`, and generic code calls it, if it's there:**

```js
function describe(zone, ZZone) {
  const first = ZZone.offset(zone, "first");
  try { .. } finally { ZZone.$dropOffset?.(first); }
}

$loudZone = { offset: loudZone_offset, show: loudZone_show, $dropOffset: noisyDrop_drop };
```

- **Beside `$drop`,** the dictionary's own type's drop: a name no Rust
  method can have.
- **A `dyn`'s dictionary has the drop of the type it names,**
  `dyn Source<Item = Noisy>`'s `$dropItem`.
- **A library's generic code calls it though the library has no
  destructor:** its consumers may, as a generic method's drops are given
  (ADR 0163).
- **A std trait's associated type, an iterator's `Item`,** has no
  dictionary: its drop is its caller's to give, as a type parameter's is
  ([ADR 0421](0421-std-associated-type-drops-are-given.md)). (Amended: it
  was an error where a type may have a destructor.)

## Why

- **It's exact:** the `associated_type_drops` corpus case drops a value of
  an associated type in generic code, in a generic struct, in an `Option`,
  at the end of a block, on reassignment and through a `dyn`, of an impl
  with a destructor and one without, with native Rust; the crates test
  drops a consumer's value in a library's generic code.
- **It's what a type parameter's drop is:** the same `?.()`, from what the
  caller already gives.

## Amendment: what the dictionary made a JS iterator of is what it drops (#978)

An associated type bound by `Iterator` that an impl gives the crate's own
iterator is a JS iterator in generic code, `$iterator(value, next)`, made
by the dictionary's method (ADR 0061), but its `$dropIter` dropped what it
was given as the struct itself: `drop at undefined` where native Rust shows
the struct's state. The two read one answer, `item_made_js_iterator`: the
drop is `held_drop_function`'s of an iterated value, which drops its `of`.
And generic code drops one as it drops a type parameter's value given its
caller's drop (`has_generic_drop`): after a consumer that doesn't close it,
`collect()`, and closed with the adapters that hold it, `rev()`; a callee
given such a type is given its drop, where an impl gives the type one whose
drop runs (`item_may_drop`). Corpus case `associated_iterator_dropped`.
