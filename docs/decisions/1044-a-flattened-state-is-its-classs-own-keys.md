# 1044. A flattened state is its class's own keys

Status: Accepted (the user's decisions, rd380, 2026-11-01). Amended
(rd533): a let chain's borrow of it is its object too. Amended (rd432):
its refusal points at the borrow. Amended: its declaration has its keys. Extends
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

A named borrow, `let state = &mut *chunk.state.borrow_mut();`, is still
refused where the `let`'s guard is counted: its `&mut` is the chunk, and a
whole write through it, `*state = Blocked { .. }`, would be `$assign`'s,
which deletes the chunk's other keys; it needs a write of the state's
keys first.

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

