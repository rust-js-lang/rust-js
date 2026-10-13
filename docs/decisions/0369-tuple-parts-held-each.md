# 0369. A tuple's parts that act are each held, not an array

Status: Accepted. Extends [0364](0364-mir-lowering.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)): how a tuple MIR
holds is written is no part of what the program can observe.

## Context

A tuple matched, `match (v.pop(), n > 3)`, or the arguments of a
`format_args!`, is a temporary MIR makes once and reads part by part,
`_3.0`, `_3.1`. Where every part did nothing and read nothing that
changes, tidy read each where its part was. Where one called, the whole
tuple stayed an array its parts were read of:

```js
const value = [v.pop(), n > 3];
if (value[0] != null) {
  if (value[1]) {
```

No one writes that. The playground's compiler, walking a path with
`match (dir.get(part)?, parts.peek().is_some())`, read `value$1[0].TAG`
where a person writes `entry.TAG`.

## Decision

**A tuple temporary read only by its parts holds each part that acts in a
`const` of its own, in order, and reads the rest where its part is:**

```js
const value = v.pop();
if (value != null) {
  if (n > 3) {
```

- **What's read before the last part that acts is held too**, as it was
  read before that part acted: `v.length` of `(v.len(), v.pop())` is
  `const length = v.length;` before `v.pop()`. Only what no call changes
  is read where its part is: a constant, a variable nothing sets again,
  and arithmetic and `Math` of them; a property read, an element, a call
  or a new object or array is held.
- **A variable that changes is read where its part is when nothing sets
  it after the tuple is made**: no statement after it, no closure, no
  getter's and setter's handle. A guard that sets it, or calls a closure
  that does, makes it held: a later arm reads the tuple's, as it was.
- Parts that act are held in the order Rust makes them, so what each does
  runs as it did; the array, made once and read by index, was the same
  values.

## Consequences

- `test/corpus/tuple_scrutinee_calls.rs` runs each case against native
  Rust: parts that call, a length read before a pop, a variable a guard
  sets and one a closure in a guard sets.
- A `format_args!` of many parts, `matrix_integers`'s, is a `const` for
  each that acts before the template, not one array read `value[22]`.
- Not yet: a part held is named by tidy's rule for temporaries, `value`,
  `value$1`, not by what the arm binds it to; and an arm's binding of it
  is a `const` of its own, `const x = value;`.
