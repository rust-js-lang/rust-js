# 0835. An object is across lines where its struct literal is

Status: Accepted. Extends [0444](0444-a-comment-is-where-its-code-is.md),
whose comments and blank lines are where the source has them, and
[0381](0381-the-apps-prettier-lays-it-out.md), whose Prettier lays out
what rust-js writes. Changes [0065](0065-format-with-oxfmt.md)'s layout of
an object: oxfmt's `objectWrap` is `preserve`, its default, not
`collapse`.

Case: N ([0262](0262-when-rust-and-js-disagree.md)): how an object is laid
out, not what it is.

## Context

Prettier keeps an object on lines of its own where its source has a line
break between its `{` and its first key, `objectWrap: "preserve"`, its
default: a person chose it. React's source has many:

```js
const elementType = {
  $$typeof: REACT_FORWARD_REF_TYPE,
  render,
};
const ReactSharedInternals = {
  H: null,
  A: null,
  T: null,
  S: null,
};
```

rust-js laid out every object on one line where it fits: oxfmt ran with
`collapse`, as the printer, not a person, chose each object's lines, and
the app's Prettier, given one line, kept it. The port's JS was
`const elementType = {$$typeof: REACT_FORWARD_REF_TYPE, render};`, though
its Rust, as rustfmt writes it, was across lines:

```rust
let element_type = ForwardRef {
    r#typeof: REACT_FORWARD_REF_TYPE,
    render,
};
```

## Decision

**An object made of a struct or a variant literal written across lines,
its `{` ending a line, is on lines of its own; one written on one line is
on one line where it fits**, as Prettier lays out an object a person wrote
either way.

- **The lowering says so, of the literal**: `aggregates.rs`'s
  `object_layout`, of a literal THIR has where the object is made, and
  `const_object`, of a `const`'s or a `static`'s, whose JS is rustc's
  value, by its HIR. Its answer, `js::ObjectLayout`, is the object's,
  `ExprKind::Object(props, layout)`, carried through tidying, linking and
  preparation as `TemplateLayout` is a template's (ADR 0066); one no
  literal makes, a representation's, `{ value }`, is on one line.
- **The printer tells the formatter**: `to_oxc` names each object across
  lines, `format.rs` writes a line break after its `{`, and none after
  another's, which oxc's codegen writes of every object of two fields, and
  oxfmt, with Prettier's default `preserve`, keeps or joins each; then the
  app's Prettier does the same with what it's given.
- **Of a literal of fields only**: a tuple struct's or variant's,
  `Placed(if a {`, has none, and a range's, `(if a { .. })..b`, rustc's
  desugaring's, nor does a macro's: each on one line where it fits.
- **An object in one across lines is across lines too where the outer is
  broken**, as Prettier breaks an object holding one that is: a variant's
  `{ TAG: "Ok", _0: {` of `Ok(Row {` is.
- **Objects only**: Prettier keeps no array's or JSX element's lines, so
  rust-js keeps none either.
- **A `const`'s literal is its own, not the literals within it**: rustc's
  value says nothing of them, which are on one line where they fit.

## Why

- **The port's JS is React's**: the Rust a port writes is rustfmt's, and
  its objects are on the lines React's are where rustfmt writes the
  literal so.
- **One owner of each part**: the lowering, which has the source, says how
  a literal is written; the printer, which has the JS, how it's written
  there, as a comment's place and a blank line are (ADR 0444).

## Consequences

- rustfmt writes a struct literal across lines where its fields are past
  `struct_lit_width`, 18 columns by default, so most objects of two fields
  or more are across lines: react.dev's `groups.push({heading: route.title,
  pages})`, one line in its source, is across lines in its port, as its
  Rust's `SubGroup { .. }` is. A project whose JS keeps such objects on one
  line sets rustfmt's `struct_lit_width`, or `use_small_heuristics`, as it
  wants its Rust written.
- Many snapshots of the corpus, the examples and the guide changed, each
  object to the lines its Rust has.
- The tests "an object is across lines where its struct literal is
  written so" and "Prettier keeps an object across lines where its struct
  literal is written so" hold it; the mutations `object-layout-ignored`,
  `const-object-layout-ignored`, `object-layout-of-a-tuple-variant`,
  `object-of-one-field-not-broken`, `objects-kept-as-printed` and
  `objects-across-lines-untold` prove them.
