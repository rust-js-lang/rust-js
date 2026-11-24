# 1022. A field of a struct JS made is read as often as Rust reads it

Status: Accepted (the user's decision, rd204); amended below, the user's decision: a binding that gives back what the program gave it, `#[rust_js::gives_back]`, gives the program's own object; and amended again, rd221: a struct the program builds is its own data, whatever JS gives of its type; and amended, #411, the user's decision: a binding that gives fresh plain data, `#[rust_js::gives_plain_data]`, gives no getter's object; and amended, #430: a pattern's subject is read once where it's tested; and amended, rd631, the user's decision: a closure's shared borrow of such a field is read where the closure derefs it; and amended, rd418, the user's decision: a cell's `replace` or `take` whose old value nothing reads is the write alone. And amended, #1908, the user's decision: an `Option` combinator's subject, which Rust reads once, is read once, of a struct JS gives an exported function too; a component's own props are plain data. Amends
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

## Amendment: what a binding gives fresh of data properties is plain data (#411)

What `JSON.parse` builds is an object of data properties alone, and so is
`Object.getOwnPropertyDescriptor`'s descriptor (ECMAScript's
FromPropertyDescriptor makes a fresh ordinary object): no getter answers
a read of it. Since the decision above, a struct such a binding gives was
taken for a JS object of getters, so React DOM's `trackValueOnNode`, ported,
held each field before its test, `if (descriptor != null) { const get =
descriptor.get; const set = descriptor.set; if (get != null) { if (set !=
null) { .. } } }`, where upstream tests `descriptor.get == null ||
descriptor.set == null` and returns; and the playground's manifest, which
`JSON.parse` gives it, kept `const runner = manifest.tests;`.

The user's decision: **one binding attribute says a result is fresh plain
data, `#[rust_js::gives_plain_data]`**: a struct the binding gives, and
each it holds, is read as plain data, as one the program builds is.
`FnPointers::js_may_make` asks it beside `gives_back`, where a binding's
call gives a result (`Made::call`). It's the binding's promise, as
`gives_back` is: what such a result holds that JS didn't build is typed as
no struct, a descriptor's `value` a `dyn IntoUnknown`. The js crate's
`object::get_own_property_descriptor` and `reflect::
get_own_property_descriptor` say it; so do the playground's `JSON.parse`
and `response.json()` bindings and todomvc's `parse`.

```rust
#[cfg_attr(rust_js, rust_js::link_name = "JSON.parse")]
#[cfg_attr(rust_js, rust_js::gives_plain_data)]
fn manifest_of(text: &str) -> Manifest {
    unreachable!()
}
```

```js
const descriptor = Object.getOwnPropertyDescriptor(node, "value");
if (descriptor == null) {
  return false;
}
if (descriptor.get == null || descriptor.set == null) {
  return false;
}
const get = descriptor.get;
const set = descriptor.set;
```

Case (ADR 0262): N, no difference: no getter answers a data property, so
reading it again gives what it gave.

Held by `test/lowering-moves.test.ts`'s "a field of fresh plain data a
binding gives is read in place" (a `JSON.parse` manifest and a
descriptor, run); mutation `plain-data-taken-for-js-made`
(`analysis/fn_pointers.ts`). The playground's snapshot reads
`manifest.tests` in place and destructures `const { root } = chosen`
again.

## Amendment: a getter's read is no panic (rd227, rd228)

The fact above has two answers. A read of a field of a struct JS made is a
getter's, so it's read as often as Rust reads it and in its order
(`Known::Getter`); but it may not panic, as Rust's read of a field can't,
so it throws only where what it's read of does (`Known::NeverThrows`), and
a guard's scope holds it in its one `finally` (ADR 0693, amended).

## Amendment: a pattern's subject is read once where it's tested (#430)

An `if let` or a `match` of a field of a struct JS made read it twice:

```rust
let view: &Margin = unsafe { js::cast(Some(js::unknown(style_prop))) };
if let Some(margin) = view.margin {
    set_margin(style, Some(margin));
}
```

printed `if (view.margin != null) { const margin = view.margin; .. }`.
MIR tests a place's discriminant and binds its payload by two reads of
the place, `discriminant(view.margin)` and `(view.margin as Some).0`,
which are one read in Rust: nothing can change the place between them,
as the match borrows it. Each was a getter's read. A guard's arm read it
four times: tested, bound for the guard, fake-borrowed and bound again.

**A pattern's subject that's a field a getter may answer is borrowed once
where it's tested, and what its patterns read of it is read through that
borrow**: `const margin = view.margin; if (margin != null) { style.margin
= margin; }`, as a borrow of the subject, `if let Some(name) =
&view.name`, already printed. `patterns.rs`'s `held_subjects` makes that
of the body's MIR before it's lowered: a subject a switch tests, its
block's last statement, of a field of a struct
`fields_answered_by_getters` says (the question `project` asks of each
read, one owner of it in `representation.rs`), and the reads of it, its
parts' tests, its bindings and its guards' fake borrows, by THIR's spans
of that match's subject and patterns, the nearest one that dominates
them. Another test of the same field, an `if let` in the arm, is a read
of its own. A while let's is read once each time round.

What a binding's body builds is no longer the program's (rd221's
`built`): a binding's body, and its closures', is native Rust's stand-in
for JS, which JS never runs. `JSON.parse`'s stand-in, building its struct
natively, had made the struct the program's, read in place.

Case (ADR 0262): R, results change; a getter's second read may give
another value or throw.

Held by `test/lowering-moves.test.ts`'s "a field of a struct JS made is
read as often as Rust reads it" (an `if let`, one inside it, and a
guard's `match`, their getters counted) and
`test/corpus/subject_of_a_struct_js_made_read_once.rs` (an `if let`, a
guard, `Some(Some(..))` and a `let else` of a binding's struct, against
native Rust); mutations `getter-subject-not-held`,
`getter-subject-fake-borrow-read`, `getter-subject-within-another-match`,
`getter-subject-other-match-region` (`lower/mir/patterns.ts`) and
`binding-body-builds` (`lower/analysis/fn_pointers.ts`).

## Amendment (2026-11-02): a closure's deref is Rust's read (rd631, the user's decision)

React's Flight reply client reads a stream part in a `catch_unwind`
closure, `reply.stringify(entry.value.flatten())`, where `entry` is the
`ReadableStreamReadResult` a reader's promise gave, a struct JS made. The
closure borrows `entry.value`, and Rust reads the field where the closure
derefs that borrow. But the field read is a getter's here, `Known::Getter`,
so `&entry.value` was a value, not a place, and the closure's maker read
it first: `const captured = entry.value; $catchUnwind(() => {
stringify(captured) .. })`. Upstream writes `JSON.stringify(entry.value,
resolveToJSON)` in its `try`.

**A closure's shared borrow of a field a getter may answer is read where
the closure derefs it**, as Rust reads it there: `$catchUnwind(() =>
stringify(entry.value))`, and a closure that derefs it twice reads it
twice, `run(() => describe(entry.value) + describe(entry.value))`. This
holds only where the field's object reads the same, a variable Rust's
borrow keeps as it is, and where the closure only derefs the borrow
(`queries::derefs_capture_only`). A closure that keeps the reference or
gives it on, `described(&entry.value)`, has it read where it's made, as
before: the callee's own read could come later than where JS reads it.
The user chose this over reading it first only for a closure called once
at once (B) and over leaving it (C).

Case ([0262](0262-when-rust-and-js-disagree.md)): R, results change. A
getter runs where Rust reads the field, each deref.

Held by `test/corpus/field_of_a_struct_js_made_captured.rs`, against
native Rust (a closure that derefs the capture twice, one in a
`catch_unwind`, and one that gives the reference on), and by
`test/lowering-moves.test.ts`'s "a closure's borrow of a getter's field
is read where it's derefed", whose getter's reads are counted. Mutations
`getter-capture-read-first` and `getter-capture-given-on-read-late`
(`lower/mir/closures.ts`) and `derefs-capture-whole-unseen`
(`lower/mir/queries.ts`).

## Amendment (2026-11-03): an old value nothing reads is no read (rd418, the user's decision)

React's useActionState overrides the URL of the form action a server
action's `$$FORM_ACTION` gives, upstream's `metadata.action =
permalink;`. Its port writes shared's `ReactCustomFormAction.action`, a
`RefCell<Option<String>>`, `metadata.action.replace(Some(..))`, and
printed `const old = metadata.action; metadata.action = '' + permalink;`.
The struct is one JS may make, so its field's read is a getter's, which
this ADR keeps; and `replace` lowered as a read of the old value, then
the write. Written `*metadata.action.borrow_mut() = Some(..)`, the same
Rust, the old value dropped where it's replaced, it printed the write
alone.

**A cell's `replace` or `take` whose old value nothing reads, and whose
drop does nothing, is the write alone**, `metadata.action = "" +
permalink;`, as the assignment through a borrow is: no getter's read
either way. Rust's program can't see the old value: it's moved out and
dropped, and the drop does nothing JS runs. What tells, MIR's: the call's
destination a temporary no statement reads and no drop reads
(`FnCx::result_unread`, the rule a `OnceCell`'s `set` already asked of
its `Result`, ADR 0847), carried to the lowering as `Call::unread`. An old
value that's read, returned, bound by the program or printed, is read
once, before the write, as before; so is one whose drop does something,
an `Option<Noisy>` whose `Drop` prints, which runs after the write, as
Rust's does.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, none. The program
reads nothing it's told; a getter on the object JS made isn't run, as
upstream's assignment runs none, and as `*borrow_mut() = v` already
printed.

Held by `test/corpus/cell_replace_dropped.rs`, against native Rust (a
`RefCell`'s `replace` and `take` and a `Cell`'s `replace` dropped, in a
closure too; an old value returned and one printed; one whose drop
prints, replaced and taken), and by `test/cross-crate-items.test.ts`'s "a
replace of a JS-made struct's cell field whose old value nothing reads is
the write alone", the port's shape across crates, whose getter's reads
are counted: none for the write, one where the old value is returned.
Mutations `replace-unread-read` (`lower/std_types/cell.ts`),
`result-unread-untold` and `result-unread-of-what-is-read`
(`lower/mir/std_calls.ts`).

## Amendment: a proxy of a Rust-made object isn't that object (2026-11-10)

A JS `Proxy` of an object Rust made, Vue's `reactive()` or an Immer draft,
is another object: a value JS made, whose fields a getter answers, as
this ADR reads it, not the object Rust made. Its identity is its own:
`===`, `Rc::ptr_eq` and `ptr::eq` of it and its target are `false`, and
where an object is stored, which the runtime records by identity for an
address asked (#1113, ADR 0562), isn't known of the proxy, so an address
asked through one is of an allocation apart from its target's. Identity
and addresses are one fact, treated alike (the user's decision, #1198,
over keeping the records as symbol-keyed properties of each object, which
a proxy would read through to its target). Case ([0262](0262-when-rust-and-js-disagree.md)):
results change where a program gives Rust both, so it's said in the
guide's differences: give Rust the object it made, Vue's `toRaw(state)`.

The object model's net, whose membrane gives the program a proxy of each
object its library gives and the library its own, so one object with two
identities, runs a case whose program or library asks an address
unproxied only (`asksAnAddress`, `test/object-model.test.ts`):
`address_of_a_field_where_its_object_is_stored.rs` printed `passed
4294963208` proxied, two allocations, where native Rust prints 16.

## Amendment (2026-11-15): an `Option` combinator's subject is read once, of a struct JS gives too (#1908, the user's decision)

A field of a struct JS *gives*, an exported function's parameter
(`FnPointers::js_may_give`, #519), stayed plain data under rd204: each
read of it isn't a getter's, as rd204 decided for what Rust reads more
than once. But where Rust reads it once and the lowering read it twice,
that was a second read no Rust program makes, which a getter answers
anew. The SDK's `scrubTo`, `let threshold = range.snap.map_or(0.0,
|snap| snap.threshold)`, printed `range.snap != null ?
range.snap.threshold : 0`, upstream's `range.snap?.threshold ?? 0`; a
`snap` getter that answered a stricter threshold the second time gave
`{ value: 1, point: undefined }`, where Rust and upstream give `{ value:
2, point: 2 }`.

The user decided (#1908): **an `Option` combinator's subject is read
once unless it's a local, a constant or a module's variable**, which no
getter answers. `Expr::reads_alike_twice` says which, and every
combinator asks it where it would test its subject and read it again
(`subject_read_twice`, `combinators/held.rs`, which `Option::map`'s in
`std_types/option.rs` asks too). Where one read does it, upstream's `?.`
and `??` are printed, `range.snap?.threshold ?? 0`, `this._text?.[key]`,
`$peek(it)?.[0]` (`optional_chain`, which now reads a key, `param[key]`,
as a property); else a `const` of it, named as a binding's read names itself, `const
count = source.count`, else as its closure names what it holds, `const s
= r.step; return s != null && s > 0.5;` (`held_subject`). Where the JS reads the held subject once anyway, first,
at the root of its chain, it's read in place again (`read_in_place`):
`window.$RB.push(a, b)` of `RB.borrow_mut().push(..)`, `$swapRemove(q.items,
0)`. A constant is read twice, `4 != null ? loud(4) :
undefined`. So is a field of the program's plain data, rd614's
`options.debugChannel.writable` (`plain_field`), but not one read of
what JS may give or make on its way: `plain_field` asks of each object
a field is read of, by its path, `p.style` of `p.style.margin`
(`given_paths`, of `fields_read_once`).

**A component's own props are plain data**, ADR 0262's second case,
the user's decision (2026-11-15, difference 59): React builds them, a
fresh object of the props' values, so no getter answers their fields,
and `p.title.map(|t| ..)` in JSX stays `{p.title != null &&
<title>{p.title}</title>}` (`FnPointers::read_anew` leaves out a
component's props struct, not what its fields hold). Only JS calling the
component itself with a getter for a prop sees it read twice;
`test/jsx-props-read.test.ts` holds that shape, a props getter read
twice and its `style`'s once.

**A `match`'s or an `if let`'s subject of a struct JS gives stays under
rd204**, read where MIR tests it and again where it binds, until #1924:
held once where it's tested, as #430 holds one of a struct JS makes, a
`const` at each subject broke react.dev's `else if` chains, llms.txt's
`extractGroupedRoutes` into a `continue` and a `let title` set twice;
#1924 is to keep the chain, its held reads ahead of it (thor agent's
decision, 2026-11-15). What Rust itself reads twice is read twice, `if
r.step.is_some() { r.step.unwrap() }`, as rd204 decided.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, as the
SDK's shows; Rust's reads are kept, however the output reads. Paid for in
output: a `const` where upstream reads a field twice, React DOM's
`const name$1 = anyProps.name; const name = name$1 != null ? '' + name$1
: undefined;`. The React port's JS changed in 13 files, 27 hunks, most of
them a `const` named as its closure names it where it was `option`,
`const debugChannel = response._debugChannel`; ReactART's
`this._text?.[key]` is as it was. The corpus's changed in 78 snapshots,
each a field read once where it was read twice, `const option =
cell.value.value`, or a `?.` where there was a `const`,
`$peek(it)?.[0]`.

`test/getter-reads/js_given_options.rs` holds it: each combinator and
`scrubTo`, reduced, of a struct an exported function is given, each
field a getter that answers another value the second time (`$reads`),
and each call's value and reads native Rust's (`//@ gives:` and `//@
logs:`, which the read cases' test now checks). The compiler before this
threw on four of them, a second read's `null`.

## Amendment: a struct only the crate constructs is no getter's (2026-11-15, the user's decision, #1950)

What an exported function takes, `&Fiber`, was read anew (`read_anew`),
JS's to give, though only the crate constructs `Fiber`, so a field tested
then read again, which MIR proves of the program's own data alone
(`nulled::field_reads`, ADR 0620, amended), kept `$unwrap(node.sibling)`
at React's 8 sites once tidy's reading of the JS went (#1950). The user
decided (Tu, 2026-11-15): a struct only the crate constructs, which JS
never builds, is plain data. The read_anew owner, `fn_pointers.rs`,
leaves out of `read_anew` each struct the crate's code, or a library's
as its contract says, makes (`own`), that no code that isn't theirs
makes (`js_may_make`), and that isn't a WebIDL dictionary
(`#[rust_js::dictionary]`), which JS builds too. ADR 0262's difference 60,
in its Consequences and the guide. React's 8 reads are plain again.

Held by test/lowering-nullable-tests.test.ts's "a RefCell field tested
and read again" (an exported struct the crate builds read as the field; one
it never builds, a getter's `Some` then `None`, panicking as Rust does; a
dictionary the crate builds keeping its check); mutations
`crate-constructed-read-anew` and `dictionary-constructed-by-the-crate`
(lower/analysis/fn_pointers.ts).

## Amendment (2026-11-18): a field of the program's own struct, held in a cell, read again (#1908's scope)

#1908 held an `Option` combinator's subject in a `const` unless it reads
alike twice or is a field of plain data, a local nothing changes. The undo
stack's `record`, upstream's `!!group && open?.group === group && time -
open.time < groupDelay`, then printed its condition as a nested
`function holds(group) { .. const open = history.open; .. }`: `history`
is an `Rc<RefCell<History>>` the closure captures, no plain local, so
`history.open` was held. And the function, which captures the guard, cost
the `remove(0)` after the push its proof (`8453fcd13`, by bisection).

A getter is what #1908 guards against: a field of what JS may make or give
(`fields_read_once`), which may answer anew. `History` is the program's
own, its fields plain properties. So a field read of a struct of the
program's whose `fields_read_once` is false, of a variable or of a field so
read, is `Known::Own`, set where the read is lowered (`project`), and an
`Own` subject is read twice, as a plain one is (rd614). ADR 0262's first
case: no getter answers it, so JS's second read gives what Rust's one gave.

Nothing a program rustc accepts writes the field between the test and the
second read, which we checked rather than assumed:

1. A closure that writes the field it's given, `slot.x.map(|v| { slot.x =
   None; v + 1 })`, compiles (`Option<i32>` is `Copy`, the closure captures
   `slot.x` apart), but a body of more than one expression is a function,
   called with the field: `map(slot.x)` reads it before the write.
2. `and_then`, `is_some_and` of the same hold it first, `const v = s.x`.
3. One expression through a `&mut` of it, `slot.x.map(|v| reset(&mut
   slot.x) + v)`, compiles too, but the `&mut` of a field is a box written
   back, more than one statement: the function again.
4. One expression that calls a closure writing it, `|v| clear() + v`, is
   refused by rustc (E0503).
5. Through a `RefCell`, the write panics, "already borrowed", natively and
   in the JS, before the second read.

So the soundness leans on the lowering of 1 and 3, a function called with
the field, and the corpus case holds both against native: a change that
inlined a body of several statements would fail there. A check of what runs
between the reads was tried and dropped: no row could reach it.

A `map_or`'s subject that isn't a variable is held, as #1908 holds one, so
its `?.` folds it (ADR 0309), `fiber.return?.tag ?? 0`, and is read again in
place where it doesn't, `q.step != null ? q.step * 2 : 0`: an `Own` subject
kept the folding the `const` gave it, and one of plain data gains it,
`guard.value?.id ?? 0`.

`record` is inline again, as upstream and as rust-js printed before
`8453fcd13`. Corpus case `option_subject_of_a_cells_field` holds the
shapes, against native; mutations `own-field-unmarked`,
`own-field-read-once-always`, `map-or-subject-not-held` and
`map-or-subject-kept-held`; `map-field-spilled`'s row is now
`untagged_field_of_each_variant`, a variant's field, which `plain` answers
and `Own`, of a struct's, doesn't. `defined()` and `present()` keep `Own`,
which says how a field is read, not what it is
(`optional-nullable-read-defined`, `defined-drops-own`): where `Own` blocked
`defined()`, React DOM's `getParent` step printed `inst?.return`, whose
`undefined` is no `None` the crate's `=== null` reads, a wrong answer
caught before it landed; corpus case `nullable_field_of_a_mapped_option`
holds `inst != null ? inst.return : null`.

## Amendment: an enum JS gives has its tag read as Rust reads its discriminant (2026-11-21, #2208)

The rule covered structs alone. `objects_held` (analysis/fn_pointers.rs)
gathered the structs an exported function is given, and `built` the
structs the crate constructs, so an enum JS gives was never a value whose
reads a getter may answer. A `match` of one tested its tag once for each
arm, `if (given.TAG === "One") … else if (given.TAG === "Two") …`. A
getter answering `"Two"` and then `"Neither"` picked the `else` arm, where
Rust, reading the discriminant once, picks `Two`.

- An enum JS holds as an object, one with a variant of fields but an
  `Option`, is held as a struct is (`objects_held`), and one the crate
  constructs, by a variant's constructor or written with named fields, is
  `built`. So `fields_read_once` answers of enums too, and one the crate
  makes stays its own data (difference 60).
- Where a value's enum may be JS's and more than one test reads its tag,
  the tag is read once, `const kind = given.TAG`, and each test is of it:
  a switch's arms (`held_tag`, representation.rs) and a drop's ladder
  (mir/drops.rs). `variant_test_by` and `variants_test_by` take the held
  tag.

Case of [0262](0262-when-rust-and-js-disagree.md): results change, so
Rust's reads are kept. Held by test/getter-reads/getter_enum_tag.rs and the
mutations `held-tag-never` and `js-given-enums-unheld`.

## Amendment: a field subject is read again only where nothing inlined between writes it (2026-11-25, #2543)

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a lost value.

A combinator's subject that is a field was read twice, once for its test
and once for its value, on the argument that an inlined closure can write
the field only through a `&mut` of it, "a call borrowck refuses beside the
read". That's not so of a `Copy` subject: `slot.x.map(..)` takes a copy, so
its closure may take `&mut slot.x`. `slot.x.map(|v| reset(&mut slot.x) + v)`
printed `Some(0)` for Rust's `Some(101)` once `reset` handed its `&mut` on
to generic code, which keeps it as a handle on the field. The closure was
then one expression, inlined with `v` read again as `slot.x`, after
`reset` had written it. `subject_read_twice` now takes the values that
run between the test and the read, the body inlined there. A field is
read twice only where none of them holds a handle on its place (or a
`dyn` pair of one); otherwise it's held in a `const` first. A closure that
isn't inlined is a function given the field, read before it runs, and is
unaffected. Held by `option_map_subject_written_through_a_kept_mut`
(reduced by `scripts/reduce.ts` from `option_subject_of_a_cells_field`'s
`observe-borrows` rewrite), that rewrite itself, and the mutation
`subject-handle-unread`.

## Amendment: a check reads a field by its descriptor; an accessor's is unchecked (#2403, the lead's decision, 2026-11-24)

A deep check that reads `object[key]` runs a getter, a read Rust never makes. `map(r)` given `r.snap`, a
getter that gives an object and then `null`, saw `null` in its body after the check had read the object:
it gave `undefined` where Rust gives `1`. That breaks ADR 1022's "read as often as Rust reads it", which ADR
1087's getter-reads cases hold. Now the runtime's checkers, `$checkField`, `$checkTag` and `$checkCell`,
read a field by its descriptor (`$checkRead`), its own or its prototypes'. An accessor's value is
`$checkAccessor`: unchecked, read once by the body, as before the checks. Only data properties are
checked. This is the limit the user decided of #1908, "accept and document the limit", not a change of
results, which ADR 0262 forbids. A `Proxy` is checked by what its target holds, which is what Immer's drafts
and Vue's reactive objects wrap, and a `get` trap that answers otherwise isn't seen. The getter-reads
harness's changing answers are real accessors now, as JS writes a getter. Held by
`test/getter-reads/js_given_options.rs` and `getter_enum_tag.rs`; the guide's semantics, 66.
