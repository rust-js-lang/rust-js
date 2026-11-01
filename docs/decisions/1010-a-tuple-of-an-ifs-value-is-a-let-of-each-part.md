# 1010. A tuple of an `if`'s value taken apart is a `let` of each part

Status: Accepted. The user's decision (2026-10-28), React port gap rd45.
Amended (2026-11-02, rd524): a tuple each branch ends making of parts it
held first.

## Context

Upstream's trackHydrated (inputValueTracking) declares two variables and
sets both in each branch:

```js
let valueField;
let expectedValue;
if (isCheckable(node)) {
  valueField = 'checked';
  expectedValue = '' + node.checked;
} else {
  valueField = 'value';
  expectedValue = node.value;
}
```

Rust's late-init form of it, `let a; let b; if .. { a = ..; b = ..; }`,
is refused by clippy's default `needless_late_init`, which asks for a
tuple of the `if`'s value, `let (value_field, expected_value) = if
checkable { ("checked", ..) } else { ("value", ..) };`. That printed an
array destructuring of a conditional of two array literals, `const
[valueField, expectedValue] = checkable ? ["checked", ..] : ["value",
..];`: two arrays upstream doesn't make, and not what a person writes.

## Decision

**A `let` that takes apart a tuple of an `if`'s or a `match`'s value,
each branch's a tuple written out, is a `let` of each part, assigned in
each branch** (tidy's `conditional_tuples`, `destructuring.rs`, after the
destructuring passes): `let a; let b; if (c) { a = x; b = y; } else { a
= z; b = w; }`, an `else if` chain of an `if`'s, as written. Each part
is given in its order, as the array's items were made, and nothing in a
branch reads the variables, declared there. A branch whose value isn't
a tuple written out, `if c { pair() } else { (1, 2) }`, keeps the array
destructuring; so does a tuple kept whole, `let p = if c { (1, 2) } else
{ (3, 4) };`.

Case ([0262](0262-when-rust-and-js-disagree.md)): None: the same values
given in the same order, and the variables nothing reads before.

## Consequences

The playground's `const [outputPaths, shown] = output.TAG === "Files" ?
[..] : [[], ""];` is two `let`s assigned in each branch. Held by
`test/corpus/tuple_of_an_if_taken_apart.rs` against native Rust, with the
order each branch's parts are made in logged, and the mutations
`conditional-tuples-kept-whole` and `conditional-tuples-else-dropped`
(`lower/mir/tidy/destructuring.ts`).

## Amendment: a tuple each branch makes last, of parts given before (2026-11-02, rd524)

React's Flight client takes a pending chunk's listeners as it resolves
it:

```rust
let (resolve_listeners, reject_listeners) = match &mut *chunk.state.borrow_mut() {
    ChunkState::Pending { value, reason } | ChunkState::PendingWeak { value, reason } => {
        (value.take(), reason.take())
    }
    _ => (None, None),
};
```

Each `take()` is statements of its own, `const old = chunk.value;
chunk.value = null;`, so the branch was no value written out, and the
tuple stayed an array made and read item by item:

```js
let tmp;
if (chunk.status === "pending" || chunk.status === "pending_weak") {
  const old = chunk.value;
  chunk.value = null;
  const old$1 = chunk.reason;
  chunk.reason = null;
  tmp = [old, old$1];
} else {
  tmp = [undefined, undefined];
}
const resolveListeners = tmp[0];
const rejectListeners = tmp[1];
```

**A temporary each branch of an `if` ends assigning a tuple written out,
`tmp = [old, old$1]`, then taken apart right after, `const a = tmp[0];
const b = tmp[1];`, and read nowhere else, is a `let` of each part
assigned where the branch made it** (`assigned_tuple`, beside
`conditional_tuples`): `let resolveListeners; let rejectListeners; if
(..) { .. rejectListeners; if (..) { resolveListeners = chunk.value; chunk.value
= null; rejectListeners = chunk.reason; chunk.reason = null; } else {
resolveListeners = undefined; rejectListeners = undefined; }`: a part its
branch held first, `const old = chunk.value;`, read only by the array,
is given where it's held. A branch that returns or throws is its guard's
by then, before the array; one that goes on otherwise, or an `if` without
an `else`, keeps the array.

Case ([0262](0262-when-rust-and-js-disagree.md)): None, as before: each
part is given in its order, after what its branch did before.

Held by `test/corpus/tuple_of_taken_parts.rs` against native Rust: a
chunk's state taken by an or-pattern, an `if`'s `take()`s, a branch that
returns and one that panics, and a part held once and given twice, which
stays held; and the mutations `assigned-tuples-kept-whole` and
`assigned-tuples-held-twice-moved`.
