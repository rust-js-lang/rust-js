# 0708. A comment before an argument, or a field, is before it

Status: Accepted. Extends [0444](0444-a-comment-is-where-its-code-is.md)
and [0658](0658-a-comment-before-a-branchs-value-is-before-it-in-the-conditional.md),
whose starts a comment is before were a statement's and a conditional's
branch's. Amended below: a `const` argument is placed where it's written;
and a field's value of an `if` or a method call is its field's; an array's item is a start too; a type's method's parameter, a class's constructor's too, is a parameter a comment is after (rd554).

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
is the one (`arm_starts`; amended below: each is placed where it's made,
and the rule is gone). A split at its line's end, after the `,`, adds
no line of nothing (`to_oxc.rs`). `test/lowering-comments-items.test.ts`,
"a comment between an array's items is before the next item".


## Amendment: a conditional is placed where its `if` is, an `if let`'s too

`branch_values` placed a conditional where its `if` is only where that was
a condition the program wrote with an `else`, so an `if let`'s value was
left where its pattern is. Since the written `if`s and `match`es were read
as the program has them (ADRs 0237 and 0879, amended), the mutation that
places every one where its `if` is, `branch-value-placed-at-pattern`,
changed no JS of the corpus (with its snapshots) or of the lowering,
snapshot, JSX, port, compiler and tidy-stage suites: `keyed`'s `// The
key, when there's one.` is before `(key != null ? key + "/" : "")` either
way. The test is gone with its mutation; `branch-value-placed-nowhere`
holds the placing.

## Amendment: a field is where its literal writes it, and a closure's lone tail's comment is its body's (2026-10-31, rd296)

React's noop renderer's host configs are struct literals of some seventy
fields, a comment or a blank line before many, their values constants,
closures and blocks:

```rust
Config {
    supports_microtasks: true,

    may_suspend_commit: |r#type| {
        // Asks whether it's possible for this combination of type and props
        // to ever need to suspend.
        r#type == "suspensey-thing"
    },

    // A comment before a field whose value is a block.
    create_instance: Box::new({
        let counter = Rc::clone(&counter);
        move || { .. }
    }),

    is_primary_renderer: true,

    get_instance_from_node: || panic!("Not yet implemented."),
    ..
}
```

Of its JS, the comment before `createInstance` was before `return {`, the
blank lines before it, before `isPrimaryRenderer` and before
`getInstanceFromNode` were lost, and `maySuspendCommit`'s comment was
before `return {` too. A field's place was its JS value's: of a block, the
closure inside it, past the `let` before it; of a constant, `true`,
nowhere; and a closure's that throws on its line, `getInstanceFromNode() {
throw .. }`, its comments given the `throw` printed first there.

- **A field is where its literal writes it** (`WrittenFields::starts`,
  `aggregates::field_starts`): its value's start in the struct literal,
  `Box::new(`'s, `true`'s, which its object's property is placed at, and
  `part_starts` and `field_docs` read. Its comments and the blank line
  before it are its own there, not a statement's its value holds on its
  line.
- **A closure's lone tail, `|t| { // c\n t.len() > 3 }`**, is its arrow's
  value, after its `=>`, where a comment before it is, though a call's
  argument starts there too, `$byteLen(t)`'s `t`; of a method, an
  object's, `maySuspendCommit(type) { return ..; }`, its `return`
  statement's, before it in the body.

The corpus is otherwise as before; React's port gains the comment
`ReactElement`'s Rust writes before `owner`, `// Record the component
responsible for creating this element.` before `_owner`, as upstream.

Not yet: a comment after a closure's parameter, `|blocked_callback, //
Profiling-only`, whose arrow keeps no parameter's place.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, a comment's and a
blank line's place.

Held by the corpus case `object_field_comments` and the mutations
`field-starts-unwritten` (`lower/aggregates.ts`),
`field-comment-to-printed-first`, `arrow-value-taken-by-a-part` and
`method-return-no-statement` (`to_oxc.ts`).

## Amendment: a static argument is placed where it's written (2026-11-01, rd421)

ReactART sets its mode with a comment above the argument, an extern
static:

```rust
js::on_load! {
    mode::set_current(
        // Change to 'art/modes/dom' for easier debugging via SVG
        FAST_NO_SIDE_EFFECTS,
    );
}
```

The comment was above the call, `// Change ..\nMode.setCurrent(FastNoSideEffects);`.
MIR reads a static through a pointer to it, `_3 = const {alloc: *const
&JsObject}`, made before the call, and the call is given `copy (*_3)`.
rd220's place for a constant (ADR 0630, amended) was by the arguments of
the call being lowered, which aren't known yet there; and its place was
the pointer's, a handle `{ value: FastNoSideEffects }`, which the read
through it drops. **A static, read as a call's argument the program wrote,
is placed where it's written, by the arguments of every call of the body,
its read's place and not only its handle's**, `Mode.setCurrent(\n  //
Change ..\n  FastNoSideEffects,\n);` as upstream (`operands.rs`'s
`argument_span`, which rd220's constant asks too; `argument_spans` is the
body's, a fact of `local_facts.rs`). A tuple's or a comparison's constant
is placed nowhere, as before.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, a comment's place.

Held by `lowering-comments-inside`'s "a comment above a static given as an
argument is above it in the call" and the mutations
`static-argument-unplaced` (`lower/mir/operands.ts`) and
`argument-spans-unknown` (`lower/mir/local_facts.ts`).

## Amendment: every constant is where it is written (2026-11-02)

`argument_spans`, which placed a constant or a static only as a call's
argument, is gone: a tuple's or an array's array is placed where its
statement is, so a comment above it is before it, and every constant the
program wrote, none a macro's, is placed where it's written
([0444](0444-a-comment-is-where-its-code-is.md), amended the same day).

## Amendment: a part that starts where its call does is what a comment is before (2026-11-02, rd617)

A comment is skipped past a call's argument whose call starts after the
comment, `// c\n f(x);`, as it's before the call, not inside it. An
argument that starts where its call does, `$f64ToInt(SPECIFIC, ..)` of
`SPECIFIC as u32`, or a receiver, `t.len()`, is the call's start too: the
comment is before what starts there, a conditional's branch, `? // c\n
$f64ToInt(..)`, or an arrow's value, `|t| { // c\n t.len() > 3 }`. That
covers the arrow's value, which had an exception of its own (rd296): it's
removed, with its mutation `arrow-value-taken-by-a-part`, which the rule
made survive. `inner-part-start-kept`'s test is "a comment after a blank
line before a method call's field keeps the line" in
`lowering-comment-fields.test.ts`, where the field's part, the outer, keeps
the blank line Prettier keeps between properties. The case is in
[ADR 0969](0969-a-variables-value-of-a-chain-of-arms-is-the-chain.md)'s
amendment of the same date.

## Amendment: an array's item is where it's made, no outer item first (2026-11-02, #479)

The rule that, of an array's first item that starts another array, the
outer's item is the one an arm starts at, was for a tuple MIR made with no
span of its own, whose start was its first part's: the inner array's
first item started where the outer item did. Since e4e258aa0 every array
and tuple MIR makes is placed where its Rust is (ADR 0444, amended), so an
inner array starts at its `[`, never at its first item, and no two arms of
arrays start at one place. Its mutation, `array-item-inner-first`,
survived the 2026-11-02 nightly; a static's, a `const`'s, a `vec!`'s and
an expression's nested arrays and tuples, a comment before the first item
or the next, printed the same JS with it. The rule is removed with its
mutation: each arm at a start is the last met (`arm_starts`), and
`check:fast --all` printed the same corpus, snapshots, guide and ports.

## Amendment: a type's method's parameter, a class's constructor's too (rd554)

React's Flight server's constructor function ends its parameters
`keepDebugAlive, // DEV-only`. Its port's `fn new(.., keep_debug_alive:
bool, // DEV-only) -> Rc<Self>`, a class's constructor (ADR 0761), had
the comment as its body's first line: where each parameter starts was
read of the module's functions alone, not of a type's methods, which a
class's constructor and methods are. And a plain type's method, its
object's property, `moved(info, by)`, mapped its parameters to no Rust,
so the comment after one was lost. **A type's methods' parameters are
each function's**: where they start is read of a namespace's methods too
(`trailing::param_starts`), and an object's method's are placed where
they're written (`params_at`), so either comment is after its parameter:

```js
function RequestInstance(
  ..
  keepDebugAlive, // DEV-only
) {
```

Case N: comments, which nothing runs. Held by `test/classes.test.ts`'s "a
class's constructor that gives an Rc of it makes this", the corpus's
`class_constructor_rc` and `test/lowering-comments-ends.test.ts`'s
`moved`; mutations `param-trailing-comment-of-a-method-after-the-line`
(`to_oxc/trailing.ts`) and `object-method-params-unplaced`
(`to_oxc/statements.ts`).

## Amendment: a closure call's arguments are where they're written (rd657)

React's Flight host dispatcher calls the previous dispatcher's function,
a `&dyn Fn` field, with a comment before its first argument:

```rust
(PREVIOUS_DISPATCHER.with(|previous_dispatcher| previous_dispatcher.preconnect))(
    /* preconnect */ href,
    cross_origin,
);
```

It printed `previousDispatcher.C(href, /* preconnect */ crossOrigin)`,
and of one argument, `/* prefetchDNS */ previousDispatcher.D(href);`, the
comment before the statement. A function's arguments are placed by MIR's
call, each of its operands where it's written; a closure's, a `dyn Fn`'s
or a fn pointer's are one tuple, `Fn::call(&f, (href, cross_origin))`,
whose items MIR keeps no place of, so `href`, a parameter read, had none,
and the comment went before the first JS after it that had one.

**A closure call's arguments are placed where they're written**, THIR's
arguments' tuple's fields, of the one call at MIR's call's place
(`closure_arguments_written`); one with a place of its own keeps it, and
none of a macro's is placed: `previousDispatcher.C(\n  /* preconnect
*/\n  href,\n  crossOrigin,\n);`, as a function's call is, its comment on
lines of its own (this ADR). Upstream writes it on one line,
`previousDispatcher.C(/* preconnect */ href, crossOrigin);`: a block
comment on its argument's line as written there is the printer's, the
same for every call, not decided here.

Case ([0262](0262-when-rust-and-js-disagree.md)): none, a comment's place.

Held by `lowering-comments-inside`'s "a comment before a dyn Fn call's
argument is before it", run, and the mutation `closure-arguments-unplaced`
(`lower/mir/calls.ts`).

## Amendment: a comment after a function's return type (rd641)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, comments only.
React's encodeReply writes `function encodeReply(value, options) /* We
don't use URLSearchParams yet but maybe */ {`; its port, `) ->
Promise<ReplyBody> /* .. */ {`, printed none: a block comment with code
after it on its line is placed after the JS of the code right before it
(ADR 0630), and no JS is of a return type. **A block comment after a
function's last parameter, its list's `)` and its return type, with its
body's `{` after it, is before that `{`, after the parameters' `)`**:
`encodeReply(value, options) /* .. */ {`. Which parameter is last is
where each ends, the parameters `comments_of` already places a line
comment after (`trailing::param_starts`, rd554); the comment is placed
from the JS of that parameter, to the first `) {` from there
(`trailing::after_params`). A function of no parameter has none to place
it by, and its comment is still lost.

Held by `test/lowering-comments-ends.test.ts`'s "a block comment after a
function's return type is before its body", run: of several parameters
across lines, of one, of no return type, and one inside the parameters,
which stays there; and the mutations `after-params-unplaced` (`to_oxc.ts`)
and `after-params-before-paren` (`to_oxc/trailing.ts`).
