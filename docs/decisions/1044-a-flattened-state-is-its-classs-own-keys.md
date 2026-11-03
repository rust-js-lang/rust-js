# 1044. A flattened state is its class's own keys

Status: Accepted (the user's decisions, rd380, 2026-11-01). Amended
(rd533): a let chain's borrow of it is its object too. Amended (rd432):
its refusal points at the borrow. Amended: its declaration has its keys.
Amended (#482): a closure's write through a borrow it captures is the
keys too. Extended by [1071](1071-a-flattened-enum-is-told-by-its-keys.md):
an enum of no tag, told by its keys, its owner `bindings::flattened`. Extends
[0204](0204-flattened-props.md), whose flattened field was a struct of
named fields or a JS object, to a `RefCell` of a discriminated union
([0284](0284-discriminated-unions.md)), and
[0761](0761-a-rust-js-class-is-a-constructor-function-and-its-prototype.md),
whose constructor takes the state's keys.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference. Rust reads a state only through its variant, so a key an
earlier variant set and the new one hasn't, which stays on the object as
upstream's chunks keep theirs, is never read by Rust (the user's decision).
A key the new variant has and its value leaves out, a `None`, is set
`undefined`, so it never reads as the old one's.

## Context

React's Flight client's chunk is one object whose `status` says what its
`value` and `reason` are: a pending chunk's listeners, a fulfilled one's
value, a rejected one's error. Upstream writes each change in place:

```js
function ReactPromise(status, value, reason) {
  this.status = status;
  this.value = value;
  this.reason = reason;
}
const chunk = new ReactPromise(PENDING, null, null);
chunk.status = INITIALIZED;
chunk.value = value;
```

The Rust an engineer writes is a class holding an enum of the states, read
by `match` and replaced whole:

```rust
#[cfg_attr(rust_js, rust_js::tag = "status")]
pub enum ChunkState {
    #[cfg_attr(rust_js, rust_js::name = "pending")]
    Pending { value: Option<Vec<Listener>>, reason: Option<Vec<Listener>> },
    #[cfg_attr(rust_js, rust_js::name = "fulfilled")]
    Initialized { value: Option<&'static Unknown>, reason: Option<&'static Unknown> },
    ..
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct ReactPromise {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub state: RefCell<ChunkState>,
}
```

It was refused at each borrow ("lending a flattened `RefCell<ChunkState>`
whole"), as a `RefCell`'s `Clone` read as one of any state. So the draft's
chunk held `status`, `value` and `reason` as cells of any value, read
through `js::cast` by its status (rd380).

## Decision

**A flattened `RefCell` of a discriminated union is its object's own
keys**: the union's tag and each key a variant has, in the order they're
first declared (`bindings::flattened_state`).

- **Its cell is a handle on the object it's in**
  (`representation::project`), of a storage of its own,
  `Storage::InPlace`, which holds the keys. A borrow reads the object, so a
  `match` reads `chunk.status`, a variant's field `chunk.value`, and a
  field written through a borrow, `value.take()`, is `chunk.value = null`.
- **A whole write sets its keys** (`Storage::written_in_place`): of a
  variant made there, its tag and each of its fields, as written, so a key
  an earlier variant set that it hasn't stays, as upstream's do; of any
  other value, each key of it, once it's computed, a key its variant
  hasn't `undefined`. Tidy writes so through a handle in place
  (`handles_in_place`), and preparation's setter of one given away does
  the same. `chunk.status = "fulfilled"; chunk.value = value; chunk.reason
  = null;`.
- **The class's constructor takes the state's keys**
  (`classes/states.rs`'s `flattened_parameter`, the one answer the constructor and each
  `new` read): `fn new(state: ChunkState)` is `function
  ReactPromise(status, value, reason) { this.status = status; .. }`, and
  `ReactPromise::new(ChunkState::Pending { value: None, reason: None })`
  is `new ReactPromise("pending", null, null)` of `#[rust_js::nullable]`
  fields, `new ReactPromise("pending")` of others, whose trailing
  `undefined`s are left out. A variable given is read for each key, `new
  ReactPromise(state.status, state.value, state.reason)`; what computes,
  given once to an arrow of each argument.
- **Never counted.** Its object counts no borrows, so one lent, `&chunk.state`,
  or borrowed while code that may borrow it runs, is refused at the field;
  `js::throw` runs none of the crate's code (`#[rust_js::runs_nothing]`),
  so a `match` that throws a state's reason holds its borrow quietly.
- **What gives or takes the value whole is refused**: `replace`, `take`,
  `swap`, `get_mut`, `replace_with`, `try_borrow`, as the value is no
  object of its own; write it through a borrow. A flattened cell of
  anything else, a struct of std's, is refused at its declaration, where
  a `RefCell` of a struct passed as one of named fields.
- `Clone`, `PartialEq`, `PartialOrd` and `Hash` read it whole only where
  rustc selects one for its type, so a `RefCell` of a state of none has
  none (`validation.rs`'s `reads_whole`).

## Why

- It's upstream's object, its keys set where upstream sets them, and the
  Rust is the enum an engineer writes, read by `match`.
- One representation: the state is its object's keys wherever it's read
  or written, a `&mut` to it given to a function too, whose box's setter
  sets each key.

## Alternatives

- **Cells of any value, `status: Cell<&str>`, `value: Cell<Option<&Unknown>>`**,
  the draft's: upstream's object, but each read a `js::cast` by status,
  Rust bent to fit.
- **The constructor given the state's object, `Object.assign(this, state)`**:
  one parameter, `new ReactPromise({ status: "pending" })`, where upstream
  gives three; the user chose upstream's.
- **Deleting a key the new variant hasn't**: what `$assign` does of an
  object; it would delete the class's own fields too, and upstream keeps
  them.

## Consequences

- The declarations (`--declarations`) don't declare the state's keys on
  the class yet.
- Held by the corpus's `flattened_state_cell`, against native Rust, of
  each variant change and a `&mut` to the state written whole; and
  `test/classes-flattened-state.test.ts`, of the Flight client's chunk and
  each refusal. Mutations `flattened-parameter-kept` (`classes.rs`),
  `flattened-state-given-whole` (`mir/calls.rs`),
  `flattened-state-missing-key-skipped` (`classes/states.rs`), `in-place-tag-left` (`js/make.rs`),
  `in-place-handle-assigned` (`tidy/values.rs`),
  `flattened-replace-allowed` (`std_types/cell.rs`),
  `flattened-state-counted-allowed` and `flattened-state-no-handle`
  (`representation.rs`), `flattened-std-struct-allowed` and
  `reads-whole-by-any-impl` (`analysis/validation.rs`), and
  `in-place-setter-assigns` (`prepare.rs`).

## Amended (rd533): a let chain's borrow of it is its object too

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference: the handle's getter gives the object, and nothing writes the
state whole through it.

The Flight client's initializeModelChunk puts its deferred listeners back
through one borrow a let chain holds:

```rust
if deferred.is_some()
    && let ChunkState::Blocked { value, reason } = &mut *cyclic_chunk.state.borrow_mut()
{
    *value = deferred;
    *reason = reject_listeners;
}
```

MIR declares a let chain's guard before its `if`, written in it, so its
handle on the chunk was a `let` when tidy's `read handles` asked for a
`const` of one, and stayed: `const borrowMut = { get value() .., set
value(value) .. }`, `const value = borrowMut.value; borrowMut.value.value
= deferred; value.reason = rejectListeners;`, one write through the
alias and one not. **Tidy reads handles again once `declarations` has
made that `let` a `const`** (`declared handles`), so each is the chunk's
key, and the `if`s join, as `if let` of the borrow alone printed:

```js
if (deferred != null && cyclicChunk.status === "blocked") {
  cyclicChunk.value = deferred;
  cyclicChunk.reason = rejectListeners;
}
```

Held by the corpus's `flattened_state_cell`'s `requeue`, against native
Rust; mutation `declared-handles-kept` (`tidy/stages.ts`).

## Amended (rd432): its refusal points at the borrow

"A flattened state lent, or borrowed where another borrow of it may be"
pointed at the field's declaration, which every such refusal in the
crate shares, so a port couldn't tell which borrow it meant. **It points
at the first place the field is lent, bound or moved, or borrowed where
another borrow may meet it** (`RefCells::loud_at`, what made the field
loud), the declaration only where no such place is known. Nothing it
compiles changes.

A named borrow, `let state = &mut *chunk.state.borrow_mut();`, was still
refused where the `let`'s guard is counted; the amendment below lifts it.

Held by `test/classes-flattened-state.test.ts`'s "a flattened state given
or taken whole, lent or held is refused", the held borrow's line; mutation
`flattened-state-refused-at-its-field` (`representation.ts`).

## Amended: its declaration has its keys

A class's `.d.ts` (`--declarations`, ADR 0761) declared its fields and
methods but not a flattened state's keys, which are its own: React's
Flight client's `declare class ReactPromise` had `_debugInfo` and `then`,
no `status`, `value` or `reason`. **Each key is a property of the class's
declaration**: the tag, any variant's literal, `status: "pending" |
"fulfilled" | "rejected"`; each other key what any variant holds there,
`undefined` of one that holds nothing there (its key is `undefined`, or
what an earlier variant left), each type once, and `unknown` or `any`
alone where one is it:

```ts
declare class Stepper {
    kind: "counted" | "named";
    n: number;
    name: undefined | string;
    label: string;
}
```

A class can't be a union, so a key's type isn't narrowed by the tag, as
the state's own type, `ChunkState`, still is. The alternative, `type
ReactPromise = ReactPromiseFields & ChunkState`, narrows, but isn't a
class, so a class whose constructor is exported would be declared two
ways; one form for every class, a property each key, was chosen
(reversible; a decision of the Flight-gaps run, logged).

Held by `test/declarations-flatten.test.ts`'s "a class's flattened state
declares its keys"; mutations `state-keys-undeclared`,
`state-key-of-a-variant-without-it-never` and
`state-key-of-any-value-a-union` (`declarations/classes.ts`).

## Amended (2026-11-02, rd432, #478): a `&mut` to it is a handle, lent or held

A wrong answer, P0 (#478), and a refusal, P2 (rd432). A `&mut` to a
flattened state was the class's object whenever its enum's every variant
has fields (`written_in_place`), so a function it was lent to that wrote
it whole, `fn finish(state: &mut ChunkState) { *state = .. }` of
`finish(&mut chunk.state.borrow_mut())`, was `$assign(chunk, ..)`, which
deleted the instance's other keys, `name` and `reads`: Node printed
`undefined fulfilled x reads 0` where Rust prints `a fulfilled x reads 5`.
An enum with a unit variant, the corpus's `Halted`, was a cell already,
so it was right only by chance. And the Flight client's named borrow, `let
state = &mut *chunk.state.borrow_mut(); .. *state = Blocked { .. }`, was
refused.

- **A `&mut` to a flattened state's enum is a handle** (`is_boxable`): the
  enums the crate's classes flatten, `bindings::flattened_states`, kept in
  the crate's facts, are cell pointees, as a value JS can't change in
  place is, whatever their variants. Lent, it's `{ value: chunk }`, and
  what it writes back is each key (ADR 0074's copy back); a variable's own
  value of the enum is a cell of it too, as a number's is. One rule, in
  representation, where `written_in_place` and `$assign` ask it.
- **A `let` of the guard's deref holds the guard** (`drops/facts.rs::let_held`,
  `guarded_borrow`): `&mut *call` or `&*call`, the guard a temporary Rust
  extends to the block's end, is held until then, `drop` of the borrow
  ending nothing, and is quiet where nothing after it asks its cell, as
  ADR 0863's `let v = c.borrow_mut();` is.
- **Its handle written whole is the object's keys**
  (`tidy/values.rs::read_handles`): a handle's `const` written whole
  through, `borrowMut.value = ..`, of a place that reads the same, is the
  handle itself where it's used, so each whole write is each key set on
  the chunk (`handles_in_place`), `chunk.status = "blocked"; chunk.value =
  listeners;`, and each read the chunk, `chunk.status === "pending"`.

Not yet: a library's flattened enum, lent in a crate that uses it, isn't in
that crate's facts.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior: every key but the state's is the instance's own, kept.

Held by corpus `flattened_state_lent_mut` (#478's repro: each variant of
fields, lent as the guard's deref and through a `let`, written whole and
in a field, and a variable's own state lent the same) and
`flattened_state_let_borrow` (rd432's: tested, taken from and replaced
whole through the `let`, the class's `name` and `reads` kept), each
against native Rust, and `flattened_state_cell`; mutations
`flattened-state-mut-the-object` (`representation.ts`),
`let-held-through-unseen` (`drops/facts.ts`) and
`in-place-handle-kept-whole` (`tidy/values.ts`).

## Amended (2026-11-02, #482): a closure's write through a borrow it captures is the keys too

Case ([0262](0262-when-rust-and-js-disagree.md)): R, results changed: a
wrong answer.

A borrow a `let` holds, `let state = &mut *chunk.state.borrow_mut();`,
written whole by a closure that captures it, `let mut set = |v| *state =
ChunkState::Initialized { .. }`, printed `chunk = { status: "fulfilled",
value: v }`, which reassigned the variable and left the instance as it
was; one tested and taken from, `if let Pending { value, .. } = state {
.. *state = Blocked { .. } }`, read the instance as a cell,
`chunk.value.status === "pending"`. Node printed `false` and `pending`
where Rust prints `true`, `blocked` and `fulfilled b`. MIR's lowering made
the handle on the state's keys right, `{ get value() { return chunk; },
set value(value) { chunk.status = value.status; .. } }`; tidy's
`read_handles` (#478) looked for a whole write through it in the body's
own statements, not its closures', so it took the handle for one only read
and gave its place, `chunk`, for each `value`, a write too. A closure's
`&mut` to the `&mut` the `let` holds is a handle on the first's `value`,
`state`, written through itself, which `read_handles` read in place
first, leaving the state's handle read at once, `const _5 = chunk`.

**A closure's whole write is one too** (`each_block_mut`, in
`read_handles` and `handles_in_place`), and **a whole write through a
handle on a state's handle's `value` is the state's** (`written_whole`
followed through each handle's place, which is made of the state's
handle then, as the body is): `chunk.status = "fulfilled"; chunk.value =
v;` in the closure, `if (chunk.status === "pending") { .. chunk.status =
"blocked"; chunk.value = listeners; }`. A handle given on from a closure,
`|v| finish(state, v)`, is the handle object, whose setter sets each key,
as before.

Held by the corpus case `flattened_state_closure_writes` (each of the
three, against native Rust), and the mutations
`whole-writes-outside-closures`, `whole-through-a-handle-unseen`,
`handle-places-unsubstituted` and `handle-root-kept`
(`mir/tidy/values.ts`); `in-place-setter-assigns` (`prepare.ts`), which
survived on `main` once #478 left no corpus case giving the handle away,
is held by the case's lent closure again.
