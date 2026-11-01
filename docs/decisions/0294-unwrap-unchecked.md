# 0294. `unwrap_unchecked()` is the value, as TypeScript's `x!` is

Status: Accepted.

Case: A ([0262](0262-when-rust-and-js-disagree.md)): a program can't
observe the difference, since a `None` is undefined behavior.

## Context

TypeScript's `x!` says a value is there and checks nothing: react.dev's
Preview hands `iframeRef.current!` to Sandpack. Rust's `unwrap()` checks
and panics, so a port with it read `$unwrap(iframeRef.current)`: a helper
and a check the original doesn't have. rust-js refused `unwrap_unchecked()`.

## Decision

**`unsafe { o.unwrap_unchecked() }` is the value itself: `o`, or of an
`Option` of an `Option`, what its box holds (ADR 0051).** It's what `x!`
means, and a port writes it where the original asserts.

```rust
let iframeElement = unsafe { iframeRef.current().unwrap_unchecked() };
// const iframeElement = iframeRef.current;
```

## Why

- **It's the JavaScript a person writes**: no check where the original has
  none.
- **It's the same program**: Rust makes a `None` here undefined behavior,
  so whatever JS does with `undefined` is within what the program means.
  A port that wants the check keeps `unwrap()`, and `$unwrap`.

## Amendment: `get_unchecked(i)` is the item

A slice's `get_unchecked(i)` is `xs[i]`, unchecked, as JS reads an item:
react.dev's NavigationBar reads `entry.contentBoxSize[0]`. Past the end is
undefined behavior in Rust, as a `None` is for `unwrap_unchecked`.

## Amendment: an `unwrap` of a cell read right after it's set is the value (rd331e)

Decided by the user (2026-11-01). ReactART's Surface sets a field and
hands it on in the next statement, `this._surface = Mode.Surface(..);
this._mountNode = createContainer(this._surface, ..)`; the port's
`Cell<Option<&ArtSurface>>`, `self._surface.set(Some(mode::surface(..)))`
then `create_container(js::unknown(self._surface.get().unwrap()), ..)`,
read `createContainer($unwrap(this._surface), ..)`: a check that can't
fail, and a helper upstream lacks. **An `unwrap()` or `expect(..)` of a
`Cell`'s `get` is the value where the statement before is a `set(Some(..))`
or `replace(Some(..))` of the same place, and the `get` is the first call
the statement runs**, all before it in evaluation order a place read, a
borrow, a literal, a constant or a cast (`body_queries::set_reads`, a THIR
fact beside `present_calls`). No code runs between the set and the read, so
nothing, another `&` to the cell that React holds say, could take the
`Some` out; a `get`'s cell is of a `Copy` value, so the set runs no drop
after it stores. Case N: the check never fires. A message that runs code,
`expect(noted(..))`, keeps `$unwrap`, which runs it.

It's narrow on purpose: a call before the read, or a statement between,
may set the cell through another reference, which only an alias analysis
could rule out, so either keeps Rust's check. A cell set in another
method, Surface's `componentDidUpdate`'s `$unwrap(this._surface)`, is
Rust's check too. Rejected: `unwrap_unchecked` in the port, an `unsafe`
assertion where Rust can see the value is there; a local first, `let
surface = ..`, a line upstream lacks.

Held by the corpus's `cell_set_then_read_back` (a `let`, a tail, `_ = ..`,
a cast, a message that runs code, another cell, a call that runs first and
unsets it, which panics as Rust's does) and `cell_set_none_then_read_back`,
and the mutations `set-read-past-a-call`, `set-read-of-a-none`,
`set-read-of-any-place`, `set-read-of-no-let`, `set-read-of-no-tail`,
`set-read-through-no-block`, `set-read-past-no-cast`
(`lower/body_queries/set_reads.rs`), `set-read-checked` and
`set-read-message-dropped` (`lower/std_types/option.rs`).
