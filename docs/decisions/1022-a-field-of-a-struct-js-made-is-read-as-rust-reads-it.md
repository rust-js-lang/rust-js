# 1022. A field of a struct JS made is read as often as Rust reads it

Status: Accepted (the user's decision, rd204); amended below, the user's decision: a binding that gives back what the program gave it, `#[rust_js::gives_back]`, gives the program's own object; and amended again, rd221: a struct the program builds is its own data, whatever JS gives of its type. Amends
[0194](0194-jsx-attributes-read-once.md),
[0672](0672-a-property-a-test-reads-is-read-once.md) and
[0866](0866-a-let-only-its-if-let-reads-is-read-there.md): a field of a
struct is plain data, read again where that reads the same, only where
the program built the struct; one JS made is a property a getter may
answer, as a binding's read is.

Case ([0262](0262-when-rust-and-js-disagree.md)): R, results would
change. A getter on an object JS made runs once for each read: read twice
where Rust reads it once, it may give another value, count its reads, or
throw. Paid for in output: a `const` of the field where it was read in
place.

## Context

React DOM's `restoreViewTransitionName`, ported, reads the style prop as a
struct `js::cast` gives, a JS object the program didn't build:

```rust
let style_prop: Option<&StyleProp> = unsafe { js::cast(js::get(props, STYLE)) };
// ..
let margin = style_prop.margin;
if margin.is_some() {
    style.set_margin(margin);
}
```

It printed `if (styleProp.margin != null) { style.margin =
styleProp.margin; }`: tidy reads in place a `let` of a place, where the
place reads the same however often it's read (ADR 0866), and a struct's
field was such a place, `Expr::has_effects` false of a member of a
variable that no binding's getter reads (ADR 0672). ADR 0194 had said a
field is read again "only of plain Rust data, never of a JS object"; but
a struct JS made is a JS object, whose `margin` may be a getter, which ran
twice where Rust reads it once (AGENTS.md: preserve side effects and
evaluation order).

## Decision

**A read of a field of a struct whose value JS may have made is a
property a getter may answer**, `Known::Getter`, as a binding's read is:
every pass that asks `has_effects`, `reads_same` or `Known::Getter`
keeps it where Rust reads it, so a `let` of it keeps its `const`.

One owner says which structs those are, the crate's analysis of what may
come from code that isn't its own, `analysis/fn_pointers.rs`, which
already follows each value from outside for the fn pointers it may hold
(ADR 0907): `FnPointers::js_may_make(struct)`, asked by
`representation.rs`'s `project` where it reads a field. A struct JS may
make is one a value code that isn't the crate's made is or holds, through
its fields, a reference, an `Option`, a `Vec` and what a function it holds
gives:

- a binding's or a library's result, `js::cast`'s too, and what a JS
  function it gives gives when it's called;
- a library's static or constant, and what a library's `Deref` gives.

A struct the program builds and never takes from outside is its own data,
read in place as before; so is what std makes of what it isn't given, a
`transmute`'s, which is the program's bytes, not a JS object. (What serde
gives was a third, amended below: a struct its `Deserialize` builds is one
the program builds.)

Three routes are left uncovered, the user's decision: each relies on JS
giving an object of plain data, no getters.

**What JS calls an exported function with.** A struct parameter of an exported function, a component's props, a class's `this`,
is read as the program's data, as before. Taking each for a JS object,
as a getter may answer, kept a `const` of nearly every field a component
or a method reads, `const step = this.step;`, and made a `Cell` field of
one a handle object, `{ get value() { .. } }.value`; React passes its
components props it made of what it was given, data properties. Where JS
calls one with an object of getters, a field read twice runs one twice.

**Nor what JS calls a function of the crate's with**, a closure's, a `fn`
pointer's or a `dyn Fn`'s parameters given to a binding: most are the
program's own objects given back, a bound function's, `function::bind(
call_destroy.react_stack_bottom_frame, ..)`'s fiber, or a state's, and
taking them for JS's kept React's `__DEV__ ? fiber._debugTask : null`
from being written so.

**Nor a generic `js::cast::<T>`.** What generic code casts is its caller's
type, and a cast of what the program stored, React's value stack's, is
its own object: such a struct is read as the program's data.

```js
const margin = styleProp.margin;
if (margin != null) {
  style.margin = margin;
}
```

## Why

The fact is the one `has_effects` already asks of a binding's read, so
no pass learns a new rule: tidy's moves, logicals and temporaries keep a
getter's read where it's written, as ADRs 0672 and 0866 say. Which
structs come from outside is the same question ADR 0907 answers of fn
pointers, over the same calls, so it's answered by that walk, not a
second one.

`test/lowering-moves.test.ts` counts a getter's reads of a field of a
struct `js::cast` gives, read once, and checks the program's own
struct is read in place. `test/corpus/field_of_a_struct_js_made_read_once.rs`
compares a binding's struct, `JSON.parse`'s, and the program's with
native Rust. Mutations take away each route and the fact.

## Alternatives

- A struct that's `js::Defined`: a class the program builds, React's
  fiber, is one too, and a struct JS made needn't be.
- Following each value from where JS made it to each read: a function's
  parameter reads what any caller gives, so it must ask the type anyway.
- Every struct: the program's own data, read in place today, would keep a
  `const` of each field read.

## Amendment: what a binding gives back is the program's

The user's decision, rd204's follow-up. React's `useState`, `useReducer`
and the other hooks give back what the program gave them, its state, a
ref's `current`, a context's value: the program's own objects, which
React keeps and never makes. Taken for JS's, a component's every read of
its state's fields was held, `const className = t.done ? "done" : "";`
before the JSX it was written in.

**A binding marked `#[rust_js::gives_back]` gives the program's own
object**, so a struct it gives isn't one JS may make: one fact on the
binding (`bindings::gives_back`), which `FnPointers` asks where it
follows a call's result, not a list of them. The react crate marks its
hooks, `use_state`, `use_reducer`, `use_reducer_with`, `use_optimistic`,
`use_optimistic_with`, `use_action_state`, `use_async_action_state`,
`use_context`, `use_memo`, `use_callback`, `use_effect_event`,
`use_deferred_value`, `use_sync_external_store` and `RefObject::current`.

What stays a JS object's is what JS made: `JSON.parse`'s, todomvc's
stored todos and the playground's fetched manifest, whose fields keep
their `const`s.


## Amendment: a struct the program builds is its own data (rd221)

rd204's rule asked, of each struct, whether code that isn't the crate's
may make a value of it, and took any `js::cast` to it for a yes. React's
port casts its own objects back from values of any shape everywhere:
`getComponentNameFromOwner`'s `js::cast::<&Fiber>(Some(owner))` of an
owner that's a fiber or a component's info, a fiber's `stateNode` cast to
its root, a node's internal key cast to its fiber in React DOM, a hook's
queue cast to the view `ConcurrentQueue` the update functions share. So the
fiber, the root, every update, context and transition, and all they hold,
became JS objects of getters: regenerating react-in-rust changed 41 files,
some 1,500 lines more unlike upstream's, 231 of them dead reads, `const
old = fiber.return;` before each `fiber.return = ..`, and a context's
`current_value` written through a handle object, `({ get value() { .. },
set value(v) { .. } }).value = currentValue`. The diff was the repro; a
two-crate probe of no cast didn't show it.

**A struct the program makes values of, by a literal in its code or a
library's, is its own data, whatever JS gives of its type, and so is what
it holds**: a JS object given back to the program through a value of any
shape is one it made. Where a cast's JS object is one the program didn't
make, reading it relies on its being plain data, as an exported
function's parameters do (above). Three parts, each in
`analysis/fn_pointers.rs`, the one owner:

- **`built`**: each struct expression's struct, in any body, a
  `Deserialize` impl's too; a struct of the crate's or of a rust-js
  library's, never a binding crate's, a webapi or js dictionary, which a
  binding gives as JS's own object. What JS may make is walked past such a
  struct, so a fiber's never-built view of its updates is the program's
  too. std's structs, an `Rc`, are walked through, never one JS made.
- **A library's contract says its `built`** (`Library::built`, library
  ABI 11, the tooling's validation too): React DOM takes the reconciler's
  fiber for the program's, as the reconciler does.
- **A cast of a value the program has is that value**: `js::cast(Some(
  js::unknown_of(Rc::clone(queue))))`, through `Some`, a borrow, an `Rc`'s
  clone and identity bindings, of a struct of the program's, is JS's only
  where that value's type may be (`FnPointers::recast`, resolved until no
  more may be). A view of a style prop JS gave is JS's still.

What stays JS's is a struct no code builds that a binding, a `js::cast`
of what may be anything, a library's static or `Deref` gives: React's
props structs, `ViewTransitionProps` from `memoizedProps`, which a
component's JSX made; a host config's result, `getSuspenseInstanceFallbackErrorDetails`'s;
a class instance; a thenable; a dictionary `Object.getOwnPropertyDescriptor`
gives. todomvc's stored todos, which the amendment above kept as JS's, are
`Todo`s the program builds too, read in place again, `<TodoItem
key={todo.id} ..>` in a `filter(..).map(..)` of the array, not a `const
key = todo.id;` in an iterator's. serde's own exemption
(`recognition::lowered_serde`) is gone: a
struct its `Deserialize` builds is `built`, and nothing else serde gives
is a struct of fields read.

Regenerated on this, react-in-rust's diff from 5944f4d is the reads of
those JS objects alone: about 150 lines in 9 files, where it was 2,500 in
41.

Held by `test/lowering-moves.test.ts`, "a struct the program builds is its
own data, whatever JS gives of its type" (a fiber cast from an owner, set
in place; its never-built view of updates; a view of a kept `Rc`; a view of
a style prop JS gave, read once, run), and `test/cross-crate-items.test.ts`,
"a struct a library builds is the program's own data in its consumer"
(`fiber.parent = parent;` of a library's fiber, not `const old` first; a
library's props JS gave read once). Mutations `built-struct-unrecorded`,
`built-struct-holds-js-made`, `recast-taken-for-js`, `recast-of-js-made-plain`,
`std-structs-js-made`, `library-built-unwritten`, `library-built-unread`.

The cost: a struct the program builds and also casts from a JS object of
getters reads that object's fields as plain data; no program has needed
otherwise. This widens the user's rd204 follow-up (plain-data reliance for
exported parameters, closures' parameters and generic casts) to a cast of
any shape where the struct is the program's; it's a decision to confirm.
