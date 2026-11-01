# 1045. An or-pattern's alternatives of one JS place are one arm

Status: Accepted (rd430, P1, 2026-11-01).

## Context

React's Flight client's chunk is a flattened state (ADR 1044), its
`status`, `value` and `reason` the object's own keys. Upstream reads a
pending or blocked chunk's listeners as one case:

```js
switch (chunk.status) {
  case PENDING:
  case BLOCKED:
    if (chunk.value === null) chunk.value = [];
    chunk.value.push(resolve);
}
```

Rust writes it as an or-pattern of the two variants' field,
`ChunkState::Pending { value } | ChunkState::Blocked { value }`, `&mut`
through the state's borrow. MIR binds `value` in each alternative's block,
`&mut ((*s) as Pending).0` and `&mut ((*s) as Blocked).0`, and goes on to
the arm. rust-js lowered each block apart: a variable each alternative
set, `let value; if (pending) { value = {get value() .., set value(v)
..} } else ..`, a getter and setter box of a `&mut` set in a branch, and
of a `match` that gives a value, a labeled block, `bb11: { .. break bb11;
}`. The six places in the Flight client printed so, which blocked its
install (no JS labels, the user's rule). A tagged enum's
`Circle { r } | Square { r }` printed the same, `let r; if (..) r =
shape.r; else ..`.

## Decision

**An or-pattern's alternative whose blocks JS holds as an earlier
alternative's is that alternative**: its switch target goes on where the
earlier one does, so the switch tests either, `chunk.status === "pending"
|| chunk.status === "blocked"`, and the arm and its binding are lowered
once. Two alternatives are one when their blocks, past empty ones and
false edges, have the same statements, each place alike but where one
reads a field of one variant and the other a field of another at **one JS
place** (`places.rs`'s `one_variant_place`, beside `mir_field`, which reads
them): a tagged enum's or a flattened state's fields of one key, of one
type, stored alike (nullable and optional alike), no cell's. An `Option`'s
variants, a write's result's and an untagged enum's aren't: each payload
is its own. They go on to one block. Each binds by reference or by copy:
one that moves a part out, `A { x, .. } | B { x, .. }` of a `String`, is
its own, as what's left of the variant drops by which part moved (a
corpus's `or_pattern_part_moves`, its `drop a1` early). And at least one
of their places differs so: two arms the program wrote alike, of no
variant's field, stay as it wrote them, React's getArrayKind's
`typeof value === "object"` arm.

`mir/alternatives.rs` finds them (`mir_alike_alternatives`); the body's
local facts take each as a guard that holds takes its arm (`held_arms`),
so the graph and the switch's cases see one target; and the locals' counts
leave the later alternative's blocks out, which JS never runs, so its
binding is written once and read in place, as one alternative's is.

```js
export function listen(chunk, id) {
  if (chunk.status === "pending" || chunk.status === "blocked") {
    ..
  }
}
```

Case: none of [0262](0262-when-rust-and-js-disagree.md)'s differences:
the JS does what each alternative does, one place's read or write.

## Alternatives

- Merging the arms after lowering, where their JS is alike: names given
  apart in each, `value` and `value$1`, make their text differ, and a
  JS comparison would reconstruct what MIR says.
- A flattened state's binding alone: a tagged enum's `Circle { r } |
  Square { r }` is the same question, `shape.r` either way.

## Consequences

The Flight client's six `case PENDING: case BLOCKED:` places print one test
of either, no box and no label. What's left reads worse than upstream and
is filed apart: the arm's binding of a flattened state is `const value =
chunk; value.value`, where upstream reads `chunk.value` (a variant's field
of a single alternative prints it too), and `value.take()` before a whole
write sets `chunk.value = null` the write sets again.

Held by `test/corpus/or_pattern_one_place.rs`, run against native Rust (a
tagged enum's, fields at other keys, `Circle { r } | Ring(r)`, kept apart,
and a flattened state's), `test/lowering-or-patterns.test.ts`'s "an
or-pattern's bindings of one place are one arm" and
`test/classes-flattened-state.test.ts`'s "an or-pattern of a field each
variant has is its key, one arm", run in Node. Mutations
`alike-alternatives-apart`, `alike-alternatives-of-any-field`,
`alike-alternatives-moved`, `alike-alternatives-written-alike`
(`mir/alternatives.rs`), `one-variant-place-of-any-key` (`places.rs`),
`unreached-alternative-counted` (`locals.rs`).
