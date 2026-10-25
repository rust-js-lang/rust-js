# 0885. A closure value is given to a JS helper as it is

Status: Accepted. Extends how a crate function given as a callback is
passed as itself to every value of a closure type, and fixes #259.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is kept. `fold(init, joined(f, g))` was
`reduce((a, b) => joined(f, g)(a, b), init)`: the call that makes the
closure ran once for each item, so a closure with state of its own started
again each time, and the closure made each time was never dropped. Native
Rust makes it once and drops it, with what it took, as `fold` ends.

## Context

A callback given to a JS iterator's helper, `map`, `filter`, `reduce`, is
wrapped, `(item) => f(item)`, so the extra arguments JS gives, an index and
the array, never reach a function that would read them: a binding, a
function pointer, what a trait's method is. A function item of the crate's
was given as itself, `items.filter(isRule)`, since its JS takes what Rust
gives it alone. Any other value, a variable holding a closure or a call
that makes one, was wrapped, and the wrapper evaluated it on each call.

## Decision

- **A value of a closure type is given as it is** (`is_called_as_written`,
  of the type revealed, an `impl FnMut` a function gives too): its JS is an
  arrow of the closure's own parameters, which never reads the rest, so
  it's evaluated once, where it's given. A function item of the crate's
  still is; a binding, a function pointer and a trait's method still are
  wrapped.
- **A closure a function gives is dropped where std drops what it's
  given**: `given_unused` reads the argument's type revealed, so the
  closure behind an `impl FnMut` carries its drop to the end of the `fold`,
  `$dispose(f)` after it, as one written in place drops what it took there.

## Consequences

- Readable too: `$mapEnds(it, (item) => value(item))` with `const value =
  (l) => l` before it is `$mapEnds(it, (l) => l)`, and std's compiled folds
  are `reduce(f, init)`; renames 716.8 to 714.2 in ten thousand lines, and
  `Iterator.from` 14.7 to 14.8, the new case's generic `fold_with`.
- Tested by `closures_made_and_folded.rs` against native Rust: a closure
  that counts, one that drops what it took, and std's `map_fold` shape in
  generic code. Mutations hold the closure given as it is and its drop.
