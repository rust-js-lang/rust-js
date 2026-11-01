# 1055. A `match` of what a `match` made is one dispatch

Status: Accepted (rd433, P2, 2026-11-02). Extends
[0364](0364-mir-lowering.md) (bodies are lowered from their MIR).

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The rewrite is MIR's
own jump threading: each arm runs what it ran, the same drops in the same
order, and goes on to the case the switch would have chosen. Each field is
read where the arm wrote it. Nothing observable changes.

## Context

React's Flight client's `then` reads the chunk's state and calls back
in one `switch`:

```js
switch (chunk.status) {
  case INITIALIZED:
    if (typeof resolve === 'function') {
      resolve(chunk.value);
    }
    break;
  case PENDING: ..
  default:
    if (typeof reject === 'function') {
      reject(chunk.reason);
    }
    break;
}
```

In Rust the state is a `RefCell`, and a callback may read the chunk again.
A borrow held across the call would make native Rust panic. So the port
copies out what's settled first, and calls back once the borrow has ended:

```rust
let settled = match &mut *chunk.state.borrow_mut() {
    ChunkState::Initialized { value, .. } => Ok(*value),
    ChunkState::Pending { value, reason } => { ..; return; }
    ChunkState::Halted { .. } => return,
    ChunkState::Errored { reason } => Err(*reason),
    ChunkState::ResolvedModel { reason, .. } => Err(Some(*reason)),
};
match settled {
    Ok(value) => if let Some(resolve) = resolve { resolve(value) },
    Err(reason) => if let Some(reject) = reject { reject(reason) },
}
```

rust-js printed the `Result` as an object built only to be tested a moment
later, so there were two dispatches:

```js
let settled;
if (chunk.status === "fulfilled") {
  settled = { TAG: "Ok", _0: chunk.value };
} else if (..) { .. }
if (settled.TAG === "Ok") { .. } else { .. }
```

## Decision

**Where a local enum's every write is a whole variant of at most one
field, and each write goes on to a switch on its discriminant, each arm
goes on to its variant's case.** Only storage, fake reads and drops of
other values may lie between the write and the switch. Those blocks are
copied to each arm, so they run as before. Each field is read as a local
of its own, written where the arm made the variant. If the case binds that
field whole to a variable, `Err(reason)`, the variable is that local:

```js
let reason;
if (chunk.status === "fulfilled") {
  const value = chunk.value;
  if (resolve != null) {
    resolve(value);
  }
  return;
}
..
if (chunk.status === "rejected") {
  reason = chunk.reason;
} else {
  reason = chunk.reason;
}
if (reject != null) {
  reject(reason);
}
```

- **Where it's decided.** `mir/threaded.rs` rewrites the MIR before it is
  lowered (`lowered_from_mir`, after `held_subjects`). It works from MIR's
  facts alone: the writes, the blocks between, the switch and the field
  reads. It never works from the JS `{TAG, _0}` shape.
- **What is left as it was.** The enum stays whole in these cases:
  - it's read whole, given back, borrowed or fake-borrowed by a guard;
  - its discriminant is read anywhere but the switch;
  - a variant has two fields or more;
  - JS drops something it holds, `Ok(Loud(..))`;
  - code runs between the writes and the switch (`let small = count <
    10;`), which would be copied to each arm;
  - its payload's parts are bound apart, `Some((text, count))`, which stay
    one array, `const [text, count] = first`;
  - anything other than a write's way reaches the switch.
- **A variant two arms make** is threaded too. Its case is then reached
  from both, after the `if` chain, as above.
- **The comments before the second `match`** are printed before the first
  case it goes on to: the ending block keeps the `match`'s span
  (`Mir::cases`, read by `mir_statements`).

## Why

- **It's upstream's JS**: one dispatch, each arm calling back with what it
  read, from the Rust a person writes to keep a `RefCell`'s borrow short.
- **It's exact.** A rustc pass of the same name is behind
  `-Zmir-opt-level=2` and runs on optimized MIR. rust-js lowers its own
  bodies from borrowck's MIR. This rewrite keeps every block, drop and
  read of that MIR, and only drops the enum value, which has no JS drop.

## Alternatives

- **Reading the `{TAG, _0}` object back in tidy**, which would fold
  `settled = {TAG: "Ok", ..}` and `settled.TAG === "Ok"`. Rejected: it would
  rebuild a decision from a JS shape. MIR already says which variant each
  arm made.
- **rustc's own `JumpThreading`.** Rejected: it runs only on optimized MIR,
  at a level that changes every body rust-js reads.
- **Threading only variants one arm makes.** Rejected: Flight's `then`
  makes `Err` in three arms. Their merge after the chain reads as the
  `default:` upstream writes. No label is needed, because the other arms
  leave.

## Consequences

- The port writes the two-step form above and gets one dispatch.
- `if let Some(v) = done` of a `match` that made `done`,
  `nested_option_inner_none`'s `settled`, reads `out.push(chunk.value)` in
  the arm. `loop_value_given_in_its_test`'s `either` returns
  `total * 10` where its `None` is made.
- Not yet: a variant of two fields or more, and a payload's parts bound
  apart. Their write would be several locals, so the statement indexes
  location-keyed facts depend on would move.

## What holds it

- Corpus `match_of_what_a_match_made`, against native Rust: Flight's
  `then`, whose callback reads the chunk again; a variant two arms make; an
  `Option` made in each arm; one in a loop; an or-pattern of two variants;
  and each case left whole above.
- Mutations (`scripts/mutations/lower/mir/threaded.ts`):
  `threaded-matches-made-whole`, `threaded-case-of-another-variant`,
  `threaded-read-whole`, `threaded-second-field-lost`,
  `threaded-drops-dropped`, `threaded-past-code`, `threaded-parts-apart`,
  `threaded-comments-lost`, `threaded-binding-apart`.

## Amendment: an `Option` of its value made with no borrow to end stays a variable (rd531, P1)

ReactART's createInstance, `let instance = if .. { let instance = ..;
Some(instance) } else if .. else { None }; let Some(instance) = instance
else { panic!(..) };`, was threaded: `let instance$1; .. { const instance =
..; instance$1 = instance; } .. else { throw .. }`, the program's name lost
and upstream's `if (!instance) throw` after the `switch` gone; and React
DOM's scrollIntoView's `host`, its `const host = .. ? fragment.host :
undefined; if (host !== undefined)` folded into the `if`. **An `Option`
that's its value or `undefined`, not a `{ value }` box, is threaded only
where what's between the two `match`es ends a borrow, a guard's drop**:
there it's the rd433 form, which lets a callback borrow again; and of a
`loop`'s value, which JS has no expression of, its breaks writing it; elsewhere
it's no object of a tag made to be tested again, but the value the program
named, which it tests after. Case N: the same JS either way, by its form.
A `Result`, or another enum, which is a tagged object, is threaded as
before. Held by the corpus's `match_of_what_a_match_made`'s
`instance_of`, against native Rust, `loop_value_given_in_its_test`'s
`either`, and the `options` snapshot's `label`, `const name = n > 0 ?
String(n) : undefined`; mutations
`threaded-unboxed-option-of-no-borrow`, `threaded-loop-value-kept` and
`threaded-unboxed-option-past-a-borrow-kept` (`mir/threaded.rs`).

