# 1147. What isn't JS yet is a hole, which no tree is printed with

Status: Accepted (2026-11-14, #1671's change 4, M1; the plan the lead
approved, with `Taken` holes in M1 and every other `undefined` named
`Expr::js_undefined()`).

Case ([0262](0262-when-rust-and-js-disagree.md)): none. This is about
rust-js's own trees, not a difference between Rust and JS. A hole left in
a tree is rust-js's mistake, and it is refused.

## Context

Review #1671's change 4 found that lowering wrote placeholders as JS's own
`undefined`. Two of them stood for something that wasn't JS yet:

- A library's item that the crate compiles itself was asked for while a
  body was lowered. `fn_ref` gave back `Expr::undefined()`, and the crate
  was lowered again with the item (ADR 0914, amended).
- A value was taken out of where it was held by
  `std::mem::replace(place, Expr::undefined())`, in 74 places, until what
  was made of it was put back.

A placeholder that was never filled printed as `undefined` where Rust has
a value. Nothing could tell it apart from the 268 other uses of
`Expr::undefined()` that meant JS's value: `None`, a unit, a missing
argument. The tree already had one hole, `ItemHole`: the place an item
written in a body goes until the pipeline puts it there.

## Decision

**`ExprKind::Hole(Hole)` is the one form of what isn't JS yet.** It has
three kinds:

- **`Item(index)`**, the former `ItemHole`: where the pipeline puts an
  item written in a body.
- **`Asked`**: a library's item asked for. The crate is lowered again
  with it, so this lowering's JS is never kept.
- **`Taken(location)`**: what `std::mem::replace` took a value out of.
  It records where it was taken, by `#[track_caller]` on `Expr::taken()`,
  so a refusal names the line.

A hole reads nothing and declares nothing. Where each kind is refused:

- `Module::unprintable` refuses every kind.
- `js::validate` refuses `Taken` in any tree. It refuses `Asked` in a
  module's tree, made once the crate's lowering is final. A body's tree
  between tidy's stages may still hold an `Asked`, because that lowering
  is thrown away.
- An `Item` hole is the pipeline's to fill and `unprintable`'s to refuse.

**JS's own `undefined` is `Expr::js_undefined()`**, and the bare
constructor is gone. Every use now says which of the two it means.
test/architecture-holes.test.ts holds this:

- no `Expr::undefined` is left;
- no `mem::replace` takes a value out with `Expr::js_undefined()`;
- `fn_ref` gives `Hole::Asked`.

There is one listed exception. `take_field` takes an item of a pending
list, `format_args!`'s or a tuple's. Where the list is still made whole,
that item's place holds JS's `undefined`, and nothing reads it:
`const [, b] = [undefined, b]`, which tidy reads as `b`. The first build
found this as the only `Taken` hole that reached printed output, in nine
compiles. None of them read the slot, so none was a wrong answer.

## Consequences

- No output changes. The corpus, the crate corpus, the pairs and the
  ports compile to the same JS.
- A placeholder that leaks now fails the compile, naming where it was
  taken, instead of printing `undefined`.
- The `Asked` hole is the basis of change 4's M2. That milestone replaces
  CrateFacts' cells with returned `Observations` and one loop, and can
  rely on no hole surviving the last lowering.
