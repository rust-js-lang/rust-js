# 0427. `#[track_caller]`: a function that reads where it was called is given it

Status: Accepted.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): where a call is, a
`Location`'s file, line and column, is what a program reads, so it's Rust's,
paid for in a parameter.

## Context

A `#[track_caller]` function is given where it was called, which
`Location::caller()` reads: a helper that panics, or logs, names its
caller's line rather than its own. std's `Index::index` is one, so every
`v[i]` of a type of the crate's calls one.

```rust
#[track_caller]
fn here() -> &'static Location<'static> {
    Location::caller()
}
let at = here();                      // line 41, column 14
```

`Location::caller()` was an error, so was rustc's
`indexing-spans-caller-location`, and three more of its tests.

## Decision

**A function that tracks its caller, and reads where it was called, takes
it: `location`, before its evidence, and each call gives it.**

```js
function here(location) {
  return location;
}
const at = here({ file: "test/corpus/track_caller.rs", line: 41, column: 14 });
```

- **A location is `{ file, line, column }`,** where rustc's
  `span_as_caller_location` puts a call: at the outermost macro's call,
  its file as rustc names it, its column counted from 1. `{}` of one is
  `file:line:column`, and `{:?}` `Location { file: "..", line: ..,
  column: .. }`, as std's; of one written where it's shown, the text itself.
- **Which functions take it is `analysis::caller_locations`'s**, once for
  the crate: one that tracks its caller, `#[track_caller]` or its trait's
  declaration's, as rustc's `codegen_fn_attrs` says, and reads
  `Location::caller()` in its own body, or calls one that takes it. One
  that reads nothing of it takes none, so `v[i]` of an `Index` of the
  crate's is the call it was. A library's manifest says it of each item it
  exports, `takes_location`, which its consumers' calls give. Every
  function's parameters and calls ask `takes_caller_location`, the one
  question.
- **A call gives its own location, or, in a function that takes one, the
  one it was given,** as rustc passes it on.
- **`Location::caller()` is the given location in the body of a function
  that takes one, and where it's written anywhere else,** a closure in it
  too: the closure's body is its own, `body_owner`, which a closure lowered
  from MIR now says too, as one from THIR did.
- **A tracked function as a value**, `.map(here)` or `here as fn()`, is an
  arrow that calls it with its definition's location, as rustc's shim of
  it does.
- **Not yet:** a method that takes its caller's location in a trait's
  dictionary, generic code's or a `dyn`'s, and a tracked trait method as a
  value, which are errors.

## Why

- **It's exact:** the `track_caller` corpus case reads a location directly,
  passed on, in a closure, through a function pointer, in a `map`, and
  shown with `{}` and `{:?}`, with native Rust. The corpus leaves the
  repository's path out of the files either names, so a case's location is
  the same on any machine. The `track_caller` pair of crates calls a
  library's from its consumer, through the manifest.
- **It's what a person writes where it matters, and nothing where it
  doesn't:** the parameter is there only where something reads it.

## Alternatives

- **Every function that tracks its caller takes one.** It's what rustc
  does, and every `v[i]` of the crate's would carry an object of where it
  is that nothing reads.
- **A location as a string, `file:line:column`.** Its parts would be parsed
  back out, and a file with a colon in it would be wrong.

## Consequences

- rustc's `tracked-trait-impls` and `tracked-trait-obj`, of tracked trait
  methods called through a dictionary, are refused with a clear error, the
  next step. `file-is-nul-terminated` reads a file's C string, which is
  addressable memory, and `track-caller-ffi` calls through FFI.
