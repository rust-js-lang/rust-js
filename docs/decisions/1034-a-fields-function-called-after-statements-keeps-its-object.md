# 1034. A field's function called after statements keeps its object

Status: Accepted (the user's decision, rd285, option A).

Case ([0262](0262-when-rust-and-js-disagree.md)): second, rare and
harmless, listed in its Consequences: the field of a struct JS made, which
a getter may answer, is read after the statements its call's arguments
need, where Rust reads it first; only a getter with effects that meet
theirs tells. First, not observable, of a struct the program made: Rust
can't change the field between.

## Context

React DOM's Float calls its dispatcher's functions with options it reads
from what it's given:

```rust
(REACT_DOM_SHARED_INTERNALS.d().preload)(
    href,
    r#as,
    Some(js::unknown_of(PreloadImplOptions {
        integrity: match js::classify(js::get(options, "integrity")) {
            Kind::String(integrity) => Some(integrity),
            _ => None,
        },
        ..
    })),
);
```

Rust reads the field, then the arguments. Where those need statements,
MIR's temporary held the function first, and it printed

```js
const L = ReactDOMSharedInternals.d.L;
const integrity = options.integrity;
L(href, as, { integrity: typeof integrity === "string" ? integrity : undefined, .. });
```

called from a variable, so with `this` undefined, where the same call with
plain arguments prints `ReactDOMSharedInternals.d.L(href, ..)`, whose
`this` is `d`, as upstream's always is (gap rd285). A JS function a field
holds (ADR 0653) that reads `this` saw another one once its call's
arguments needed statements.

## Decision

**A function a field holds, called with arguments that need statements,
is called through its object: the object held, `const d =
ReactDOMSharedInternals.d; ..; d.L(href, ..)`, or read in place where it's
a variable, `d.L(href, ..)`**, as upstream writes it, its `this` the
object as the plain call's is.

Only a field Rust can't change while the body runs
(`js::Known::Lasting`): of a struct a lasting reference is to, a `&T`
parameter, which its caller's borrow freezes until the body returns, or a
`&'static T` a call gives, which nothing can borrow `&mut` again, each
written nowhere else in the body (`LocalFacts::lasting`); and no cell,
which its methods change. A field of what a `&mut` is to, of a borrow of a
local, or of a parameter assigned again is read where Rust reads it, as
before: `(inner.f)({ inner.f = triple; n })` calls `double`.

A function a with_this function may be is still held and called with no
`this`, as Rust calls it (ADR 0972, amended): it's held under its field's
name, never MIR's temporary this reads.

- `mir/local_facts.rs`'s `mir_lasting` says which locals are lasting;
  `mir/places.rs`'s `mir_place` marks a field read of one `Lasting`;
  `mir/tidy/moves.rs`'s `held_receivers`, a tidy pass after `inlined in
  values`, holds the object of a temporary of such a read that's called
  once.

## Alternatives

- **`const d = ..; const L = d.L; ..; L.call(d, href, ..)`**: Rust's order
  and upstream's `this`, which no one writes.
- **Keep `const L = ..; L(..)`**: Rust's order, `this` lost; upstream's
  dispatchers don't read it, so no React test differs, but a function JS
  gives that does sees `undefined`.
- **The argument statements made before the callee is read**: moves every
  call's callee past what its arguments do, not only a lasting field's.

## Consequences

- ADR 0262's Consequences list the exception, and the guide's differences
  (`semantics.md`, 35): a getter that answers `L` is run after the
  argument statements, where Rust runs it first.
- Held by `test/lowering-held-receivers.test.ts`: Float's call, a
  parameter's, a plain call's, run with a `this`-reading function; a
  `&mut`'s and a cell's read first; and a with_this function's held. And
  the corpus's `field_functions_called_after_arguments`, against native
  Rust: a `&'static` a call gives, a parameter, a `&mut`, a borrow of a
  local, a parameter assigned again and a cell. Mutations
  `lasting-of-any-call`, `lasting-parameter-rebound`
  (`mir/local_facts.ts`), `held-receivers-off` and
  `held-receivers-of-any-field` (`mir/tidy/moves.ts`). No mutation of the
  cell's check: no program copies a cell's value out of its place to call
  it, so none tells, but the fact must hold of every read it marks.
- The port writes `(REACT_DOM_SHARED_INTERNALS.d().preload)(..)` as it
  does; each of Float's calls whose options need statements holds the
  dispatcher and calls its function through it.
