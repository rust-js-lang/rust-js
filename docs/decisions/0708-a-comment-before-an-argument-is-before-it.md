# 0708. A comment before an argument, or a field, is before it

Status: Accepted. Extends [0444](0444-a-comment-is-where-its-code-is.md)
and [0658](0658-a-comment-before-a-branchs-value-is-before-it-in-the-conditional.md),
whose starts a comment is before were a statement's and a conditional's
branch's. Amended below: a `const` argument is placed where it's written.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: comments, which
nothing runs.

## Context

React's `describeNativeComponentFrame` says why it may write `name`, in
the arguments it's written in:

```js
Object.defineProperty(
  RunInRootFrame.DetermineComponentFrameRoot,
  // Configurable properties can be updated even if its writable descriptor
  // is set to `false`.
  // $FlowFixMe[cannot-write]
  'name',
  {value: 'DetermineComponentFrameRoot'},
);
```

Its port has the comments in `object::define_property`'s arguments, above
`"name"`. A comment is before the start that follows it, a statement or
a branch (ADR 0444, 0658), with nothing between but where code opens: an
argument was none, so the next start was the next statement, past the
call's `;`, and the comment was lost. So was one before an object
literal's field, `// What a stack trace names it.` above `value: ..`.

## Decision

**A call's argument, a `new`'s, and an object literal's field are each a
start a comment before it is before, on lines of their own, as Prettier
lays them out.**

```rust
object::define_property(
    js::unknown(&determine_component_frame_root),
    // Configurable properties can be updated even if its writable descriptor
    // is set to `false`.
    // $FlowFixMe[cannot-write]
    "name",
    &PropertyDescriptor {
        value: Some(&"DetermineComponentFrameRoot"),
        ..Default::default()
    },
);
```
```js
Object.defineProperty(
  DetermineComponentFrameRoot,
  // Configurable properties can be updated even if its writable descriptor
  // is set to `false`.
  // $FlowFixMe[cannot-write]
  "name",
  {
    value: "DetermineComponentFrameRoot",
  },
);
```

- **Where its JS starts**: an argument's span, and a field's value's,
  which is the field's in the JS (`to_oxc::part_starts`); oxc writes the
  comments attached there before the argument and before the field, as
  it does a statement's.
- **No blank line of its own**: a part's comments are written as they
  are; Prettier keeps no blank line between arguments.
- **What's first on its line is a statement's alone**: a part's start
  doesn't make the statement after it on its line not its line's first.

## Why

- **Every comment of the Rust is in the JS** (ADR 0444), here where it's
  in React's.

## Consequences

- `{ value: "DetermineComponentFrameRoot" }` is laid out across lines
  where upstream's is one, as oxfmt lays out an object it's given on
  several; the port's `..Default::default()` is no field of the JS.
- A comment before an argument that isn't one in the JS, `&frame` of
  `js::string::replace(&frame, ..)`, `frame.replace(..)`'s receiver, is
  where oxc writes none: lost, as before.

## What holds it

- `test/lowering-names.test.ts`, "a comment before an argument or a
  field is before it": the port's `define_property`, its comments before
  `"name"` and one before a field.
- Mutations `argument-comment-unplaced`, `field-comment-unplaced`.

## Amendment: a `const` argument is placed where it's written

ReactFiberLane's `markSpawnedDeferredLane(root, spawnedLane, // This
render .. NoLanes)`: the argument is a `const` read by its name, which
`named_const` gave no place, so it was no start, and the comment was
before the call, the call on one line. **A `const` read by its name is
placed where it's written**, a start as any argument is: the comment is
before it, on lines of their own. Held by `lowering-comments`'s
"ReactFiberLane's comments in an else, an if's test and a call are where
it writes them" and the mutation `named-const-unplaced` (`lower.ts`).

## Amendment: a comment before a parameter is before it

React's `createPortal` says of its third parameter, in its list:
`containerInfo: any,\n // TODO: figure out the API for cross-renderer
implementation.\n implementation: any,`. Its port writes the comment
there too, and lost it: a parameter was no start, and JS's parameters
mapped nowhere. **A function's parameter is a start a comment before it
is before, on lines of its own, after the `(` or the `,` before it**,
as a conditional's branch is (ADR 0658); Prettier then lays the list out
a parameter a line, as upstream's is. Each parameter of a function whose
JS takes them as it writes them, one each, maps where it's written
(`js::Function::param_spans`, `to_oxc::arm_starts`, `Cx::params_at`);
one whose JS adds or takes apart its parameters, a closure's, a method's
dictionaries after them, has none of its own yet. Case N: comments,
which nothing runs.

Held by `test/lowering-comments.test.ts`, "a comment before a parameter
is before it", run in JS, with its parameter's line in the source map,
and the mutations `param-spans-unknown` (`lower/mir.ts`),
`param-comments-unplaced`, `param-comment-first-after-a-comma`
(`to_oxc.ts`) and `param-mapped-nowhere` (`to_oxc/statements.ts`).
