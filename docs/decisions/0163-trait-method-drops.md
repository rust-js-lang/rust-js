# 0163. A trait's generic method is given drops for its own type parameters, as its trait declares them

Status: Accepted. Amends [0098](0098-destructors.md) and
[0106](0106-generic-traits.md).

## Context

A generic function is given a drop for each type parameter a caller gives
a value with a destructor (ADR 0098): `consume(noisy, noisyDrop_drop)`. A
trait's generic method wasn't. Called through a dictionary, its caller
knows only the trait, not which impl runs, nor what that impl drops. So a
generic trait method was an error in any crate where a type may have a
destructor: one of its own, or any library's at all.

That stopped most crates. Of 16 that shared models use, it stopped 9
(docs/crate-corpus.md): hashbrown's, smallvec's, anyhow's, semver's,
regex-syntax's, arrayvec's, num-traits' and bitflags' generic methods.

## Decision

**A trait's generic method is given a drop for each of its own type
parameters that isn't `Copy`, as the trait declares it:** the declaration,
which the caller through a dictionary knows, and which each impl has, so
both agree, as a supertrait's key does (ADR 0106).

```js
function fill(store, SStore) {
  SStore.put(store, noisy, noisyDebug(), noisyDrop_drop);
}

function logStore_put(log, item, TDebug, dropT) {
  try { .. } finally { dropT?.(item); }
}
```

- **After its own bounds' dictionaries:** a caller through a dictionary
  gives the method's arguments, its own dictionaries, then its own drops;
  one with nothing to drop is none, left out at the end.
- **An impl's method takes them,** at its own type parameters' places,
  and so does its dictionary's entry, which passes them on.
- **Called on the impl itself,** as any generic function is, it's given
  them as its type parameters are known there.
- **A library's takes them though it has no destructor:** its consumers
  may give it one.
- Both refusals go: of a generic trait method where a type may have a
  destructor, and of one given a value with a destructor.

## Why

- **It's exact:** the `generic_method_drops` corpus case compares where a
  value with a destructor is dropped, given to a generic method directly,
  through a dictionary and through a default, with native Rust; the crates
  test a library's method given a consumer's value, both ways.
- **It's what a generic function is:** the same `dropT`, in the same place.
