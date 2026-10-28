# 0696. A constructor is a `fn` item, declared where it's written

Status: Accepted; amended below: in the block it's written in, not the
innermost one that uses it; amended by
[1009](1009-a-constructor-is-held-as-a-typed-class.md): it's held as a
`js::Class`, `Class::of(&fake)`, not as a value of any shape. Amends [0308](0308-local-functions.md),
whose function written in a body was declared in its body's own list,
not the block it's written in, and without its comments.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: a function written
in a block captures nothing, so it's the same function in any block
that holds each use of it; the program can't tell.

## Context

React's `shared/ReactComponentStackFrame.js` makes a constructor where
it uses it, reads its `prototype` and constructs it:

```js
if (construct) {
  // Something should be setting the props in the constructor.
  const Fake = function () {
    throw Error();
  };
  Object.defineProperty(Fake.prototype, 'props', { set: function () { throw Error(); } });
  ..
  Reflect.construct(fn, [], Fake);
```

A closure prints as an arrow, which JS can't construct and which has no
`prototype`. A `fn` item written there was a function declaration, but
in the closure's own list, after the `if`, as its first statement after
it was none: `if (construct) { .. } function Fake() { .. }`, far from
where it's written; and the comment above it was gone.

## Decision

**A constructor Rust writes is a `fn` item: a function declaration,
which JS constructs and gives a `prototype`, declared where it's
written, in the block it's written in, after its comments.**

```rust
if construct {
    // Something should be setting the props in the constructor.
    #[cfg_attr(rust_js, rust_js::name = "Fake")]
    fn fake() -> Option<&'static Unknown> {
        panic!();
    }
    object::define_property(unsafe { js::cast::<&Unknown>(js::get(js::unknown(&fake), "prototype")) }, "props", ..);
```
```js
if (construct) {
  // Something should be setting the props in the constructor.
  function Fake() {
    throw new Error("explicit panic");
  }
  Object.defineProperty(Fake.prototype, "props", {
```

- **The block it's written in**, where it holds each use: JS scopes a
  declaration in a block to it, so one whose uses lowering put outside
  that block's statements is declared in the innermost block that holds
  each use. One used nowhere, or written after each statement of its
  block, is declared in its body's list, as before.
- **Its comments are above it**: the declaration is where its `fn` is,
  so a comment before it is before it (ADR 0444).
- **Its name is a constructor's where it's named one**: `rust_js::name
  = "Fake"`, as any function's JS name is.
- **Not a closure**: a closure is an arrow, which a constructor isn't;
  `js::new` of one throws a `TypeError` (ADR 0695).

## Why

- **The JS a person writes**: the constructor in the block that makes
  it, with its comment, not after what uses it.
- **A `fn` item is the Rust of a function that's a value of its own**:
  it captures nothing, as upstream's `Fake` doesn't; a closure that
  printed as `function` where it's constructed would be two forms of one
  closure, and one given away, to a `dyn Fn`, can't be told.

## Alternatives

- **A closure printed as `function () {}` where it's constructed or its
  `prototype` read**: a closure's form decided by its uses, which a
  closure given to another function hides.
- **Each such function at its body's top**: the block it's written in
  is lost again.

## What holds it

- `test/js-object.test.ts`, "a constructor is a fn item, declared where
  it's written": a constructor in its function's list and one in an
  `if`'s, with its comment, constructed and its `prototype` read, run.
- `test/js-object.test.ts`, "React's component stack frame is found as
  upstream finds it": `Fake` in the `if (construct)` block.
- `test/lowering-functions.test.ts`, "a function written in a body is
  declared in it", where `big` is now in its `if`.
- Mutations `written-fn-in-its-body-list`, `written-fn-comment-lost`.

## Amendment: the block it's written in, not the one using it

It was declared in the innermost block that held each use of it, so one
written at a body's top and used in one `if` moved into that `if`:

```rust
pub fn cleanup(on: bool) -> Option<fn()> {
    fn noop() {}
    if on { Some(noop) } else { None }
}
```
```js
export function cleanup(on) {
  if (on) {
    function noop() {}
    return noop;
  }
}
```

A declaration stays where it's written (least surprise): the block it's
in is the one `local_functions` found it in, and the list it's declared
in is the outermost of those holding each use that has a statement
written after it in that block. `cleanup` declares `noop` first, and
React's `Fake`, written in `if (construct)`, is in that `if`, as
before. Nothing that must hold moved `noop`: the innermost list was
the simplest test of "its block" that `Fake` passed.

- `test/lowering-branches.test.ts`, "a return of undefined is a bare
  return": `noop` at `cleanup`'s top, its `if` a bare `return noop`.
- `test/lowering-functions.test.ts`, "a function written in a body is
  declared in it": `twice` at its body's top, used in one `if`.
- Mutation `written-fn-in-the-block-using-it`.

## Amendment (2026-10-28): held as a `js::Class`

[1009](1009-a-constructor-is-held-as-a-typed-class.md): the constructor
is held as a `js::Class`, the one way: `js::new(Class::of(&fake), ())`,
`Class::of(&fake).prototype()`, not `js::new(js::unknown(&fake), ())` and
`js::cast(js::get(js::unknown(&fake), "prototype"))`, each refused naming
the form. One closed over what it captures is `function::with_this`'s,
`Class::of(function::with_this(..))`; `Class::of` of a closure, an arrow,
is refused. Its JS is as above: `function Fake() {}`, `new Fake()`,
`Fake.prototype`.
