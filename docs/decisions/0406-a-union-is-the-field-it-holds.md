# 0406. A union is the object of the field it holds

Status: Accepted. Amends [0390](0390-addressable-memory-is-refused-as-such.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)) for a field read
through the field it holds, or one of the same type, whose bytes are the
same; D for a field read as another type's bytes, which JS has none of:
rust-js's error where it runs, as which field a union holds is known only
then.

## Context

ADR 0390 refused a union as addressable memory. But most of what a
program does with one reads the field it wrote: a tagged value,
`Value { tag, number: Number { int } }`, read by its tag, a union with a
method, a `Display`, a field written in place, a pattern of it. Only a
field read as another type's, `U { int: 1 }.float`, reads bytes. rustc's
union tests were refused whole.

## Decision

**A union is the object of the one field it holds, `{ int: 10 }`. A
field read is `$unionField(u, "int")`, what it holds there, or under a
field of the same type, `$unionField(u, "left", "right")`; one it doesn't
hold is rust-js's error where it's read. A field written is
`$unionSet(u, "int", v)`, the one it holds now.**

```js
const n = { int: 1 };
$unionSet(n, "float", 0.5);
console.log(`${$displayF64($unionField(n, "float"))}`);
```

```
rust-js does not support reading a union's `float` while it holds `int`, which reads its bytes as another type's
```

- **The decision is where it runs**, as a lock re-locked by its own
  thread is (ADR 0328): which field a union holds depends on what ran, as
  a tag says, `union-pat-refutability.rs`'s, so a compile error would
  refuse a program that's right.
- **A field of the same type** reads the same bytes, exactly: `$unionField`
  is given them. A field of no bytes, `()`, MIR never reads.
- **A copy is a union of its own**, `{ ...u }`, and of the one it holds,
  a copy of what changes in place in it.
- **A union's own `Drop` runs; its fields are never dropped**, which Rust
  holds to what has no destructor or a `ManuallyDrop`, which is what it
  holds: `ManuallyDrop::new(x)` and `*md` are `x`.
- **To TypeScript**, a union of one-field objects, `{ int: number } |
  { float: number }`.
- **Only the crate's own**: std's `MaybeUninit`, a union, is what it holds
  (ADR 0332).
- **Still refused**: a `&mut` to a union's field, whose write would change
  which it holds unsaid.

## Why

- **It's the program Rust runs**: the corpus's `unions.rs` reads a tagged
  value by its tag and by a pattern, writes a field as another, through a
  `&mut` too, reads fields of one type, copies a union and changes the
  original, holds a struct changed in place, and drops a union with a
  `Drop` of its own and a `ManuallyDrop` field, each with native Rust;
  `union_const.rs` reads a constant's two fields of one type. A runtime
  test refuses a read as another type, and a field read after another's
  written. Eleven of rustc's union tests pass; those that read bytes as
  another type's stop with the error.
