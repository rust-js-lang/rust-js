# 0437. A scope's drops on a panic are one `try { .. } finally { .. }`

Status: Accepted. Extends [0098](0098-destructors.md) and
[0372](0372-a-check-that-drops.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)): the same drops run, in
the same order, on every path; only how the JS reads changes.

## Context

A panic drops what's live as it unwinds (ADR 0098). MIR gives each call
that may panic its own cleanup, so rust-js wrapped each statement that may
throw while a value with a destructor is live in its own `try`, whose
`catch` drops it and throws on, and the scope's end dropped it again:

```js
const shapes = [..];
try {
  console.log(total(shapes));
} catch (error) {
  for (const item of shapes) item.impl.$drop?.(item.value);
  throw error;
}
try {
  console.log("end");
} catch (error) {
  for (const item of shapes) item.impl.$drop?.(item.value);
  throw error;
}
for (const item of shapes) item.impl.$drop?.(item.value);
```

Each drop was written three times over, and the corpus's programs with
destructors were mostly this.

## Decision

**A run of statements a value's drop ends, each of which that may throw
drops it last in its `catch`, is one `try { .. } finally { drop }`**, as a
person writes cleanup:

```js
const shapes = [..];
try {
  console.log(total(shapes));
  console.log("end");
} finally {
  for (const item of shapes) item.impl.$drop?.(item.value);
}
```

- **Inside out**: a value made inside the run is its own scope inside it,
  `try { try { .. } finally { drop(e) } } finally { drop(d) }`, so a panic
  drops the newest first, as Rust's unwinding does. A `catch` left with
  nothing but its `throw` is no `try`.
- **The same drops**: a statement's `catch` drops the scope's value last,
  and the scope's end drops it, the two written alike up to where each was
  written and what each names what it declares (`js::stmts_alike`), so
  `for (const item$1 of v)` is `for (const item of v)`. It runs once the
  temporaries are named (`tidy::named`).
- **Only a scope**: a run stops at a statement that may throw outside any
  `try`, and at one whose `catch` drops another value last, one whose life
  crosses this one's, `drop(first)` while `second` is live, which keeps its
  `try` per statement. What the run declares and what follows reads is
  declared before it; a `let x;` only it reads is declared where it's made.
- **Two checks no program reaches**: a `try` whose body returns, and one
  whose `catch` drops another value last before this one's end. Lowering
  makes neither (it wraps no `return`, and a crossing value's drop is
  wrapped itself); each stays, so a `finally` never drops twice if that
  changes. They have no mutation, as nothing reaches them.

## Why

- **It's the code a person writes**, and it reads in one place what a
  scope cleans up. 97 corpus programs lost about 6,300 lines.
- **It's the program Rust runs**: every corpus case still matches native
  Rust, the panicking ones (`drop_on_panic`, `drop_index_panic`,
  `drop_replace_panics`, `folded_call_panic_drops`) too, and
  `scoped_drops_cross` holds two crossing lives and a panic after.

## Alternatives

- **`using` declarations**, `[Symbol.dispose]`: the newest idiom, but not
  in every engine rust-js targets.
- **One `finally` that drops a drop's flags**: also covers crossing lives,
  at the cost of a flag per value a reader has to follow.
