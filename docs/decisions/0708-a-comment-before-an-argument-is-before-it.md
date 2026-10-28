# 0708. A comment before an argument, or a field, is before it

Status: Accepted. Extends [0444](0444-a-comment-is-where-its-code-is.md)
and [0658](0658-a-comment-before-a-branchs-value-is-before-it-in-the-conditional.md),
whose starts a comment is before were a statement's and a conditional's
branch's. Amended below: a `const` argument is placed where it's written;
and a field's value of an `if` or a method call is its field's; an array's item is a start too.

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

## Amendment: a comment after a call's last argument is before its `)`

React's defaultOnUncaughtError writes `console.warn('%s\n\n%s\n',
componentNameMessage, errorBoundaryMessage, // We let our console.error
wrapper ..\n);`, the comment after the last argument. Its port writes it
there too, in `console::warn((..))`'s tuple; it was no start's, so it was
before the next statement, the call on one line, and the blank line the
Rust has before the call was lost with it, the comment now the
statement's. **A comment on lines of its own after a call's or `new`'s
last argument, only what closes the call after it, `,` and `)`, is before
the call's `)`** (`to_oxc/arguments.rs`): attached at the call's end's
last byte, where oxc writes a comment before the `)`, on lines of its
own, as Prettier keeps it; of the innermost such call, `f(a, g(b, // c\n))`
`g`'s. The blank line before the statement is as the Rust has it, as
before. Case N: comments, which nothing runs.

One after an array literal's last element, `&[a, b, // c\n]`, is still
before the next argument: oxc's array printer writes no comment between
or after its elements, a gap for the printer, not this rule.

Held by `test/lowering-comments.test.ts`, "a comment after a call's last
argument is before its parenthesis", run, the guide's `warn`, and the
mutations `argument-end-comment-unplaced`, `argument-end-of-any-comment-inside`
and `argument-end-of-the-outermost` (`to_oxc/arguments.ts`).

## Amendment: a comment after a struct literal's last field, and after a parameter

React's ReactPerformanceTrack and ReactFiberCacheComponent write a
comment after an object literal's last property, `{ name, start, // We
don't know the owner. }`, and one after a parameter on its line,
`startTime: number, // DEV-only`. Their ports write them there too, and
both moved: the object's was before the `return`, as a comment inside a
statement nothing places is before it ("never lost"), and the parameter's
after the `{`, the last JS of its line, which oxfmt made the body's first
line.

- **A comment on lines of its own after a struct literal's last field,
  only `,` and its `}` after it, is before the object's `}`**, on lines of
  its own (`to_oxc/closing.rs`). oxc's printer writes no comment there, so
  the printed line is split where the `}` is, by the map, as a
  conditional's branch's is (ADR 0658); one that's first on its line has
  no line of nothing before it.
- **A line comment after a parameter on its line, past its `,`, is after
  that parameter's JS**, the rest of the line, `) {`, on the next, as a
  comment after a parameter the next one follows already was (ADR 0733):
  `start, // DEV-only`, Prettier then laying the list out a parameter a
  line (`trailing::after_param`).

```js
export function track(
  name,
  start, // DEV-only
) {
  return {
    name,
    start,
    // We don't know the owner.
  };
}
```

Case N: comments, which nothing runs. Held by
`test/lowering-comments-ends.test.ts`, run, and the guide's `track`; the
mutations `object-end-comment-unplaced`, `object-end-comment-of-any-code-after`,
`object-end-comment-printed-twice` (`to_oxc/closing.ts`),
`object-end-comment-a-line-of-nothing-before` (`to_oxc.ts`) and
`param-trailing-comment-after-the-line` (`to_oxc/trailing.ts`).

## Amendment: a field's value of an `if` or a method call is its field's

React's `getSuspendedCache` returns, after a blank line,

```js
  return {
    // We must also save the parent, so that when we resume we can detect
    // a refresh.
    parent: isPrimaryRenderer
      ? CacheContext._currentValue
      : CacheContext._currentValue2,
    pool: cacheFromPool,
  };
```

whose port reads the parent through a thread-local, `parent:
CACHE_CONTEXT.with(|cache_context| if IS_PRIMARY_RENDERER { .. } else {
.. }).unwrap()`. Its comment was before `return {`, the blank line
before it lost: the field's start wasn't one, so the comment went to the
statement it's inside, from where its line is. Two things took it:

- **A conditional tidy makes of an `if`'s branches was placed nowhere**:
  `x = c ? a : b` kept the `if`'s span on its statement, not its value,
  which a struct literal's field reads once its `const` is read in place.
  The conditional is now placed where the `if` is (`branch_values`), as
  what's written.
- **Two parts that start at one place**, the field's value, `$unwrap(..)`
  of a method call, and that call's first argument, its receiver: the
  inner took the start, a call the comment is before, not inside, which
  the comment skips. The outer's is kept (`part_starts`), whose whole the
  comment may be inside.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, a comment's place.
Held by `test/lowering-comment-fields.test.ts`, "a comment before a field
of an if's value or a method call's is the field's", and the mutations
`branch-value-placed-nowhere` (`mir/tidy/branches.rs`) and
`inner-part-start-kept` (`to_oxc.rs`).

## Amendment: a comment between an array's items is before the next item

React's getAttributeAlias writes its aliases as a `Map` of pairs, a comment
among them, `['acceptCharset', 'accept-charset'], // HTML and SVG
attributes, ..\n['crossOrigin', 'crossorigin'],`; its port's
`HashMap::from([.., // ..\n("crossOrigin", "crossorigin"), ..])` lost the
comment (React port gap rd17), and one between a `vec!`'s items went
before the statement.

**An array's item is a start, as a conditional's branch is** (ADR 0658):
a comment before it, only space between, is before it on lines of its
own, after the `[` or the `,` before it, as Prettier writes it. A tuple
MIR made has no span of its own: its start is where its first part is,
and of an array's first item that starts another array, the outer's item
is the one (`arm_starts`). A split at its line's end, after the `,`, adds
no line of nothing (`to_oxc.rs`). `test/lowering-comments-items.test.ts`,
"a comment between an array's items is before the next item".

