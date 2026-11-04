# 1044. A flattened state is its class's own keys

Status: Accepted (the user's decisions, rd380, 2026-11-01). Amended
(rd533): a let chain's borrow of it is its object too. Amended (rd432):
its refusal points at the borrow. Amended: its declaration has its keys.
Amended (#482): a closure's write through a borrow it captures is the
keys too. Amended (2026-11-04, #658, #659, the user's decision): what's
moved out of it whole is a copy of its variant's keys. Amended (#687):
its cell's `swap`, `get_mut`, `replace_with`, `into_inner` and
`try_borrow` compile. Amended (#699): its cell's own `take` compiles,
its `Default` made first. Amended (#700): its guard's drop drops nothing,
beside a counted cell of its type too. Extended by [1071](1071-a-flattened-enum-is-told-by-its-keys.md):
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
- **What gives the value whole or swaps it was refused**: its cell's
  `swap`, `get_mut`, `replace_with`, `into_inner`, `try_borrow`, as the
  value is no object of its own. What's moved out of it whole, its cell's
  `replace` and `mem::replace`, `take`, `swap` and
  `Cell::from_mut(..).replace` of a `&mut` to it, is a copy of its
  variant's keys (the amendment of 2026-11-04 below), and so is each of
  those five's (#687, below). A flattened cell of
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
handle then, as the body is; since 2026-11-04, the handle on its `value`
is the state's handle itself, below): `chunk.status = "fulfilled"; chunk.value =
v;` in the closure, `if (chunk.status === "pending") { .. chunk.status =
"blocked"; chunk.value = listeners; }`. A handle given on from a closure,
`|v| finish(state, v)`, is the handle object, whose setter sets each key,
as before.

Held by the corpus case `flattened_state_closure_writes` (each of the
three, against native Rust), and the mutations
`whole-writes-outside-closures` (`mir/tidy/values.ts`;
`whole-through-a-handle-unseen`, `handle-places-unsubstituted` and
`handle-root-kept` went with what they held, below); `in-place-setter-assigns` (`prepare.ts`), which
survived on `main` once #478 left no corpus case giving the handle away,
is held by the case's lent closure again.

## Amended (2026-11-04, #658, #659): what's moved out of it whole is a copy of its variant's keys

A wrong answer, P0 (#658), and a refusal, P2 (#659). A `&mut` to the
state lent to a function is a box of the class's object, `{ value: task
}` (#478), so a function that moved it out, `fn finish(state: &mut
Status) -> Status { mem::replace(state, Status::Done { .. }) }` of
`finish(&mut task.state.borrow_mut())`, gave back `state.value`, the
object itself, which the copy back then overwrote: Node printed `Done {
value: "v" }` where Rust prints `Pending { tries: 1 }`. The same move out
through the guard in one body, `mem::replace(&mut *state, ..)`, `take`,
`swap` and `Cell::from_mut(&mut *state).replace(..)`, gave `const old =
task;` the same way, so #636 refused it, and the cell's own `replace` was
refused for the same reason (rd380).

**What's moved out of it whole is a copy of its variant's keys, chosen
by its tag** (`mut_ref.rs`'s `moved_out`, ADR
[1086](1086-a-mut-refs-js-form-is-mut-ref-forms.md)'s owner of a
`&mut`'s form): the copy is only the keys of the variant the state is
in, so it never carries a key an earlier variant left on the object, and
it matches a value made in JS (the user's decision, 2026-11-04). Then
each key of what's written is set on the object, as before.

```js
const old =
  task.status === "pending"
    ? { status: task.status, tries: task.tries }
    : task.status === "done"
      ? { status: task.status, value: task.value }
      : { status: task.status };
task.status = "done";
task.value = "w";
```

- **One rule, at the move out.** `mir_swap_or_replace` copies of a
  `&mut` to a flattened state that's not a place of the body's own (a
  variable of its own holds a value, which a write rebinds), and so does
  `Cell::from_mut(..).replace`'s `moved_out`; the cell's own `replace` reads
  its handle's object and sets each key of the new value there
  (`Storage::written_in_place`). A function the state is lent to copies it
  where it moves it out, as its `&mut` is to the flattened enum. The lend
  is unchanged, `{ value: task }`: lent to generic code, a `&mut T`, it's
  refused already at the borrow, which that code may meet, so no callee
  that moves it out apart, `$apart`, ever has the object. What the lend
  gives stays React's `{ value: chunk }`, which no callee copies unless it
  moves the state out.
- **A swap is two moves out, never a destructuring.** `mem::swap(&mut
  *state, &mut h.f)` was JS's `[a, b] = [b, a]` of two places, whose
  target through the guard's handle is the class's object, `[task, h.f] =
  [h.f, ..]`, which rebinds the variable; of a flattened state it's `const
  t = ..copy..; task.status = h.f.status; ..; h.f = t;`.
- **What's handed on is the guard's handle.** `&mut *state` given to
  `Cell::from_mut`, whose cell is then given to a function, or to a
  function that gives the `&mut` back, was a handle on the handle's
  `value`, whose place is the class's object: `{ get value() { return
  task; }, set value(value) { task = value; } }`, a write the caller
  never saw (#678, P0, on `main` before too). A handle on a flattened
  state's handle's `value` is that handle (`read_handles`), whose setter
  sets each key. It's the one rule of a handle on the state's handle's
  `value`: #482's three, a whole write through one followed to the
  state (`written_whole` through each handle's place), the state's
  handle put in each such handle's place, and a handle whose place's
  root is a handle read again, are gone, as the corpus's
  `flattened_state_closure_writes` runs as before without them. And
  `Cell::swap` of
  one, `Cell::from_mut(&mut *state).swap(..)`, is two moves out, each a
  copy, not `$cellSwap`'s copy apart of the class's object.
- **Variants of the same keys are one copy**, tested together, `chunk.status
  === "pending" || chunk.status === "fulfilled" ? { status, value, reason
  } : ..`, and a state whose variants all have the same keys is one copy,
  untested. The copy reads the tag from the object, `status:
  task.status`, so variants that share a copy share its object.
- **Inline, not a helper.** A helper, `$variantKeys(task, "status", {
  pending: ["tries"], done: ["value"] })`, is shorter for many variants,
  but names keys as strings and reads as nothing a person writes; the
  ternary is what the user's decision wrote, and React's Flight chunks'
  variants share `value` and `reason`, so theirs are one or two copies.
- Still refused: the cell's `swap`, whose `$refCellSwap` moves each side
  apart, `$apart`, which would carry the class's other keys, and
  `get_mut`, `replace_with`, `into_inner` and `try_borrow`. The cell's
  own `take` compiles where its borrow can't meet another; `Default`'s,
  the crate's code, runs while it's borrowed, so it's refused as a borrow
  another may meet, as before. #636's record of what holds a guard
  (`Made::flattened`, `through_flattened`) is gone with its refusal.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior: the copy is the value moved out, apart from the object.
A key the copy leaves out, an earlier variant's, is one no Rust code
reads, and leaving it out is what a value made in JS has.

Held by corpus `flattened_state_moved_out` (each of the four through the
guard, each lent to a function that moves it out, the cell's own
`replace`, and a variable's own state lent the same) and
`flattened_state_swapped_places` (swapped with a struct's field and a
`Vec`'s item, through the guard and lent) and `flattened_state_handed_on`
(`Cell::from_mut`'s cell swapped, taken, set and given to a function, and
the `&mut` given to a function that gives it back), against native Rust; `test/classes-flattened-state.test.ts`'s "what's moved out of a
flattened state whole is a copy of its variant's keys", each copy's keys
alone in JS, after a write of one variant over another; and the `&mut`
matrix's `flattened_state__field`, whose `heldRefused` is gone and whose
program now lends the state to a function too. Mutations
`flattened-moved-out-the-object`, `flattened-copy-of-every-key`,
`flattened-copy-untested` (`representation/mut_ref.ts`),
`flattened-moved-out-as-it-is` and `flattened-swap-destructured`
(`mir/entries.ts`),
`flattened-cell-replaced-as-any-cells`,
`flattened-cell-replaced-old-the-object` and `flattened-cell-swapped-apart`
(`std_types/cell.ts`), and `handle-on-a-state-handles-value-kept`
(`mir/tidy/values.ts`).

## Amended (#687): its cell's `swap`, `get_mut`, `replace_with`, `into_inner` and `try_borrow` compile

A refusal, P2 (#687). The cell's own calls that give its value whole or
swap it were refused with this ADR's message, as the value is no object
of its own; #658/#659's copy of its variant's keys made a move out of it
right, and each of these is that move out, or the handle a borrow is.
Each is built on what's there:

- **`into_inner` moves it out**: a cell moved whole out of a field that
  holds its value is a cell of what it held (`whole_property_cell`), a
  flattened state's a copy of its variant's keys (`mut_ref.rs`'s
  `moved_out_of_cell`), as the class's object may be written again. The
  field given to its own `into_inner`, or to `get_mut` through the `&mut`
  rustc holds alone, isn't read whole (`validation.rs`), nor lent
  (`ref_cells.rs`'s `borrows_its_cell`).
- **`get_mut` lends the place**: its handle on the class's object, which
  a `&mut` to the state is (#478), held as a guard's is, a `const` tidy's
  `read_handles` reads as the object and whose whole writes set each key;
  so is `Cell::from_mut`'s of the guard's `&mut`. Lent on, it's `{ value:
  task }`, written back key by key, as a guard's `&mut` is.
- **`replace_with(f)` moves it out**: `f` is given the handle, through
  which it writes the keys, then what it held is a copy of its variant's
  keys (`moved_out`) and each key of what `f` made is set, as std's
  `mem::replace` of the `&mut` it lends. Its borrow, held while `f` runs,
  is momentary where nothing `f` runs can ask of it (`drops/facts.rs`'s
  `replaced_with`, by `CellBorrows::bounds_ask`, which takes a body that
  may borrow anything, or what rust-js can't see, as asking), so it's
  never counted; one whose `f` may borrow the cell stays refused at the
  borrow, as a borrow another may meet.
- **`swap` is two moves out**: each side a copy of its variant's keys,
  each written with the other's; a plain `RefCell` of the state, a
  parameter's or a variable's, read and written as its `value`, checked
  where its borrows are counted. Two flattened states are told apart by
  their objects, `if (a === b) throw new Error("RefCell already
  borrowed")`, as Rust's second `borrow_mut` panics of one swapped with
  itself, so a swap of one counts nothing and lends neither cell
  (`flattened_state_lent`). Which of the two cells are flattened states
  is MIR's, `QuietCells::flattened`, carried by `Call::flattened`, as
  either may be held in a variable by then.
- **`try_borrow` is the guard**: a `try_borrow`'s guard is momentary where
  a borrow's would be (`momentary` of its `Result`), not counted always,
  so of a cell no borrow of which can meet another it's `Ok` of the
  guard, as no other can be held; a flattened state's, its handle, held
  as `get_mut`'s is. Its `Ok` read at once is its parts, `{ TAG: "Ok",
  _0: state }.TAG` `"Ok"` (`Expr::member`, of an object made here whose
  other fields do nothing), so `try_borrow_mut().is_ok()` is `true`.

A wrong answer on `main` (P0), which `try_borrow` and `replace_with`
counting their cell always had hidden for theirs: a guard of one cell
held across another's `replace_with` whose function borrows the first,
`let held = b.borrow(); a.replace_with(|n| { b.borrow_mut().push('!');
*n + 1 })`, was quiet, as a cell's own call ran nothing but its borrow
(`Called::Cell`), so `b` was uncounted and the JS wrote it where Rust
panics. **A cell's own call runs what its bounds say too**,
`replace_with`'s function, a `take`'s `Default`
(`CellBorrows::cell_call_asks`, which `call_asks` and tidy's `Quiet` ask,
and `call_asks_seen`), a body no summary says asking, so that guard is
counted. And tidy's whole write through a handle, of a value it
names, names each its own, `state`, `state$1`, as two such writes may be
in one list.

Not in this: the cell's own `take` of a `Default` of the crate's stays
refused, as the borrow analysis reads that `Default` as run while the cell
is borrowed, though std runs it first; a follow-up.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior: each value moved out is apart from the object, and a
swap of a state with itself panics as Rust's does.

Held by corpus `flattened_state_cell_swapped` (two classes' states, a
state and a plain cell, a parameter's and a variable's, either way
round), `flattened_state_cell_swapped_itself` (run-fail),
`flattened_state_got_mut`, `flattened_state_into_inner`,
`flattened_state_replaced_with` (its `held_across` a guard of another
cell its function's `try_borrow_mut` sees), `flattened_state_tried`,
`flattened_state_cell_swapped_borrowed` (run-fail, a plain cell borrowed
while it's swapped) and `replace_with_held_across` (run-fail, the P0),
against native Rust; and `test/classes-flattened-state.test.ts`'s copies, whose
`replace_with`, `swap` of a plain cell and of another chunk's state, and
`into_inner` each have their variant's keys alone, and the refusal of a
`replace_with` whose function borrows the state. Mutations in
`std_types/cell.ts`, `drops/facts.ts`, `drops/ref_cells.ts`,
`mir/local_facts.ts`, `mir/operands.ts`, `analysis/cell_borrows.ts`,
`analysis/validation.ts`, `mir/tidy/values.ts` and `js/make.ts`.

## Amended (#699): its cell's own `take` compiles, its `Default` made first

A refusal, P2 (#699), and two wrong answers on `main` (P0, filed and
closed with it). The cell's own `take` of a `Default` of the crate's, a
derive's too, was refused (#687's "not in this"): `counted_in`
(`drops/ref_cells.rs`) counted a `take`'s borrow wherever its `Default`
wasn't std's (`std_default`, ADR 0585), as one held while the crate's
code runs, and a flattened state counts no borrows. That premise is
false: std's `RefCell::take` is `self.replace(Default::default())`, so
`Default` runs before the cell is borrowed, and a `take`'s borrow is
held for a moment, as a `replace`'s is. The same premise made lowering
put the borrow first: a counted cell's `take` was `$take($borrowMut(log),
logDefault_default())`, and a `Cell`'s `old = $apart(count.value);
count.value = countDefault_default();`, so a `Default` that prints
printed nothing before a borrowed cell's panic, where Rust prints, and
one that writes the cell it's taken of was moved out of before it
wrote, `Count(1)` where Rust's is `Count(11)`.

- **A `take`'s borrow is never counted**, as a `replace`'s isn't: the
  `std_default` case is gone with recognition's `std_default`. What its
  `Default` runs still asks of a borrow held across the `take`
  (`CellBorrows::cell_call_asks`, whose `bounds_ask` says so, before the
  cell's borrow or while it's held alike), so a guard of the cell held
  across one is counted and the `take` throws as Rust's panics. What
  meets the cell's own borrow is only what runs while it's held, a
  `replace_with`'s function (`replaced_with`), never a `take`'s
  `Default`.
- **A `take`'s `Default` that may run code is made first**
  (`std_types/cell.rs`), `const next = statusDefault_default();`, before
  the cell's borrow is checked and what it held is read, then moved out
  and written as before: of a flattened state, the copy of its variant's
  keys and each key set; of a counted cell, `$take($borrowMut(log),
  next)`. The cell before it where reading it runs code, a thread-local's
  made where it's first used, `const cell = local();`, as std's `with`
  makes the cell before its `take` runs `Default`. A `Default` that runs
  nothing, std's or a derive's of such, `{ n: 0 }`, stays in place.

```js
const next = statusDefault_default();
old =
  task.status === "pending"
    ? { status: task.status, tries: task.tries }
    : { status: task.status, value: task.value };
task.status = next.status;
task.tries = next.tries;
task.value = next.value;
```

A field's `RefCell` whose only borrows were `take`s is now its value, as
one only `replace`d is (ADR 0585): `const old = bag.items; bag.items = {
n: 0 };`, and `refcell_transparent`'s newtype's borrows are unchecked.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior: what's printed before a panic, and what's moved out
once `Default` has written the cell.

Held by corpus `flattened_state_taken_default` (a derived `Default`, and
a written one that reads and writes the same state through a
thread-local, no borrow held, so neither panics), `cell_taken_default_first`
(a counted `RefCell`, one of an object, a `Cell` and a thread-local made
where it's first used, each `Default` printing and writing its cell) and
`cell_taken_default_borrowed` (run-fail: `Default` prints, then the
borrowed cell's `take` panics), against native Rust;
`test/lowering-cells.test.ts`'s "a field's RefCell written by replace and
take is its value". Mutations `take-held-while-default-runs`
(`drops/ref_cells.ts`, for `take-default-trusted`, gone with what it
mutated), `take-default-made-after` and `take-cell-made-after-default`
(`std_types/cell.ts`).

## Amended (#700): its guard's drop drops nothing, beside a counted cell of its type too

Readability, P3 (#700). Where no other `RefCell` of the state's type is
counted, a flattened state's `try_borrow_mut` read as the keys, `if
(task.status === "pending") { .. }`; beside one that is, each of its
`try_borrow`s kept its `Ok` as an object of the handle, `const value$1 =
{ TAG: "Ok", _0: state }; if (value$1.TAG === "Ok") { .. value$1._0.value
.. }`. A guard of a cell no borrow of which is counted
(`QuietCells::guards`) releases nothing, and its drop was skipped where
it's lowered (`drop_place`), but the drops' elaboration
(`mir_elaboration`) didn't know it: where the guard's type drops, as
another cell of it is counted, the `Result` holding it, partly moved by
`Ok(state)`, had a flag, so it was declared first, `let _2;`, which
neither `Expr::member`'s fold of an object made here nor tidy's
`read_handles` reaches.

**An unchecked guard's drop is no drop, by the elaboration**
(`mir/drops.rs`, given `QuietCells::unchecked_guards`, which
`mir_function` makes before it): never there to drop, so no flag says it
and it reads nothing. An unchecked guard is one of a cell no borrow of
which is counted, a quiet field's (a flattened state's) or one whose flow
says so: the value itself. A checked one, of a momentary borrow of a
counted cell, `$borrow(c)`, keeps its elaborated drop, which `drop_place`
still skips as it releases nothing, so its variable keeps its check
where it's made; leaving its drop out too moved checks into the reads,
`const items = $borrowMut(queue); items[at] = task;`, a change of its own
not made here. So the flattened state reads as where no cell of its
type is counted:

```js
const task = new Task("pending", 1, undefined, "task");
console.log(`read ${statusDebug_fmt(task)}`);
if (task.status === "pending") {
  task.tries = (task.tries + 1) >>> 0;
}
console.log("free true");
```

and so do other unchecked guards beside a counted cell of their type: a
quiet field's `for (const item of from.items)`, where its `Vec` was held
as `const items = from.items;` and indexed, and `fiber.return != null ?
fiber.return.tag : 0`, where it was `const return$ = fiber.return;`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference: the guard's drop released nothing either way, and it's the
value itself; only whether it's held in a variable changed.

Held by corpus `flattened_state_tried`, beside a counted plain cell of
the state, and `held_borrow_of_own_cell`, against native Rust, and their
snapshots; `test/lowering-field-cells.test.ts`'s "a struct's RefCell field
holds its value whatever else has its type". Mutations
`unchecked-guard-elaborated` (`mir/drops.ts`) and
`checked-guard-unchecked` (`mir/local_facts.ts`); `quiet-guard-released`,
`drop_place`'s skip, now of a checked guard alone, is held by
`refcell_vec_bounds`'s snapshot.
