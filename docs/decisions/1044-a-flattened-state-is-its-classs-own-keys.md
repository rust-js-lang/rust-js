# 1044. A flattened state is its class's own keys

Status: Accepted (the user's decisions, rd380, 2026-11-01). Extends
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
