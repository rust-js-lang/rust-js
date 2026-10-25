# 0649. `#[inline(never)]` is `/** @noinline */`

Status: Accepted. Extends [0444](0444-a-comment-is-where-its-code-is.md),
whose doc comment is its item's JSDoc. Decided with the user, 2026-10-23.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the output's layout:
an annotation a minifier reads, which changes nothing a program does.

## Context

React keeps `testStringCoercion` out of line, so the stack trace of what
it throws points there, where a comment explains it:

```js
/** @noinline */
function testStringCoercion(value) {
  // If you ended up here by following an exception call stack, here's what's
  // happened: ..
  return '' + value;
}
```

Closure Compiler, which builds React, reads `@noinline` as Rust reads
`#[inline(never)]`: keep this function out of its callers. The port wrote
`#[inline(never)]`, and rust-js dropped it.

## Decision

**A function marked `#[inline(never)]` is written with `/** @noinline */`
before it**, a method of a type's object too; where it has a doc comment,
`@noinline` is its JSDoc's last line, one JSDoc. And rust-js keeps it a
function of its own: not a forwarding function's variable (ADR 0610), nor
the argument a function giving each variant its own name is (ADR 0264).
`recognition::is_noinline` is the one answer; `js::Function::noinline`
carries it to preparation and printing.

**`#[inline]` and `#[inline(always)]` say nothing**: Closure has no
annotation for either, and a JS engine inlines what it finds hot.

## Why

- **The same meaning, the user's decision**: Rust's and Closure's
  annotations both say to keep the function out of line, and React's
  stack trace needs it.
- **One JSDoc**, as an editor shows a JS caller one.

## Alternatives

- **`@noinline` of `#[inline]` too, or `@inline` of `#[inline(always)]`**:
  Closure has no `@inline`, and `#[inline]` asks the opposite.
- **A JSDoc of its own beside the docs**: two comments where one says both.

## Consequences

- A function marked `#[inline(never)]` that forwards to a thread-local's
  function stays a function, `return get(n);`, where it was the variable.
- The corpus's `drop_guard_given_whole.rs`, whose `#[inline(never)]` keeps
  a guard's drop in its own frame, prints `/** @noinline */` too:
  `docs/readability.txt`'s JS is a line longer, 89,402 to 89,403.

## What holds it

- `test/emission.test.ts`, "a function marked #[inline(never)] is
  @noinline": one with a comment in its body, a doc of one line and of
  two, `#[inline]` and `#[inline(always)]` unmarked, a forwarding function
  and a method giving each variant its own name, each kept.
- Mutations in `scripts/mutations/lower/mir.ts`, `to_oxc.ts`, `prepare.ts`,
  `lower/patterns.ts` and `lower/recognition/std_items.ts`.

## Amendment: a function written in a body is `@noinline` too

React's `UnknownOwner` keeps a frame of its own above it, which React's
owner stack pops ("Pop the JSX frame", `shared/ReactOwnerStackFrames`), so
the frame left is `UnknownOwner`'s:

```js
/** @noinline */
function UnknownOwner() {
  /** @noinline */
  return (() => Error('react-stack-top-frame'))();
}
```

A Rust port can't write that call: clippy refuses a closure called where
it's made (`redundant_closure_call`), and rust-js writes one as its body
(ADR 0581), so its frame would be gone. The one way is a function written
in the body, marked `#[inline(never)]`, as a function of the module is.
rust-js kept it a function, but printed no `/** @noinline */`: only an
item's and a method's start were asked (`noinline_starts`).

**A function written in a body, however deep, a closure's or a module
load's too, is `/** @noinline */` where it's marked `#[inline(never)]`**:

```rust
#[inline(never)]
pub fn unknown_owner() -> &'static js::JsError {
    #[inline(never)]
    fn react_stack_top_frame() -> &'static js::JsError {
        js::js_error::new("react-stack-top-frame")
    }
    react_stack_top_frame()
}
```

```js
/** @noinline */
export function unknownOwner() {
  /** @noinline */
  function reactStackTopFrame() {
    return new Error("react-stack-top-frame");
  }
  return reactStackTopFrame();
}
```

Case N, as above: an annotation a minifier reads. The frame is there
either way; what differs from upstream is its name, `reactStackTopFrame`
where upstream's arrow has none, in the frame React's owner stack pops,
and the `@noinline` before the function rather than before the `return`
of its call.

- `test/emission.test.ts`, "a function marked #[inline(never)] is
  @noinline": React's `UnknownOwner`, its inner function's comment kept.
- Mutation `noinline-nested-unwritten` (`to_oxc.ts`).
