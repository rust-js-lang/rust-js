# A std trait's dictionary has its associated types' drops

Status: Draft, the change on disk in `src/lower/drops/types.rs` and
`test/corpus/operator_output_drops.rs`, finished. Would extend
[0178](../0178-associated-type-drops.md), whose impl's dictionary has its
associated types' drops, to a std trait that has a dictionary; and
[0421](../0421-std-associated-type-drops-are-given.md) and
[0489](../0489-a-yielded-item-is-given-its-drop-by-its-own-type.md), whose
std trait with no dictionary has its drops given by the caller.

Case: C ([0262](../0262-when-rust-and-js-disagree.md)): the value is
dropped where Rust drops it, once.

## Context

Generic code holds a value of a std trait's associated type: `a + b`'s
`<T as Add>::Output`. `Add` has a dictionary, `TAdd`, given by the caller
with `add` in it. A crate trait's dictionary has `$dropOffset` for each
associated type with a destructor (ADR 0178); a std trait's associated
type was taken to have no dictionary, so `Output` fell to no rule, and
was refused where a type may have a destructor: "a value of an associated
type". `Iterator::Item` has no dictionary, and its drop is given apart
(ADRs 0421, 0489).

## Decision

**A std trait's associated type's drop is where its dictionary is: an
impl of `Add`, `Mul`, `Index`, `Deref` and every std trait a dictionary
is given for has `$dropOutput` beside `add`, as a crate trait's impl has
`$dropOffset`; a std trait no dictionary is given for, `Iterator`,
`IntoIterator`, has its drops given by the caller, as before.**

```js
$loudAdd = { add: loudAdd_add, $drop: loudDrop_drop, $dropOutput: loudDrop_drop };

function sum_drop(a, b, TAdd) {
  const _x = TAdd.add(a, b);
  TAdd.$dropOutput?.(_x);
  return 0;
}
```

- **One question, one owner**: `traits::has_dictionary_of` says whether a
  trait's dictionary is given for a self type, and `item_drop_of` asks it:
  a dictionary, the drop is in it; none, `drop_slot` gives it. The diff on
  disk computes the answer and doesn't ask it: every std associated type,
  `Iterator::Item` with them, would be read as dictionary-held, against
  ADRs 0421 and 0489. The gate is the change.
- **The slot's name is the associated type's**, `$dropOutput`, `$dropTarget`,
  as ADR 0178 names it, a name no Rust method can have.
- **A number's dictionary has none**: `{ add: (a, b) => (a + b) | 0 }`
  drops nothing, and `?.()` is the call.
- **The refusal goes**: the diagnostics test on disk removes it; the
  limitations page never listed it, so nothing else changes there.

## Why

- **It's std's**: `operator_output_drops` sums two values with a
  destructor through a generic `T: Add`, and two numbers, with native
  Rust. Mutations: a `$dropOutput` left out of the dictionary, the gate
  reversed so an `Item` is looked for in a dictionary that hasn't it.
- **It's the one rule a reader guesses**: a trait with a dictionary keeps
  its own drops; one without is given them.

## Alternatives

- **A caller-given slot, `dropTOutput`**, as `Iterator::Item` has: one
  mechanism for every std associated type, but `Add` already has a
  dictionary, so its drop would travel beside it, two carriers for one
  trait. Two ways; rejected.

## Consequences

- **What compiles grows**: a generic operator's result where a type may
  have a destructor, and itertools' and num-traits' generic arithmetic
  over such types.
