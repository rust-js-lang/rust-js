# 1032. An `if` whose branches are alike is its branch

Status: Accepted. Extends tidy's alike branches, a conditional's (`logicals.rs`'s
`conditional`), to an `if`'s statements.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. The test decides
nothing Rust sees: either branch does the same. Its read of a global a
host may lack throws in JS, where Rust's `match` can't fail, so the JS
without it is closer to the Rust.

## Context

React DOM's FragmentInstance keeps its observers, `Set<IntersectionObserver
| ResizeObserver>`, and calls each one's own method,
`observer.observe(child)`. Its port holds the union as TypeScript types it,
an untagged enum of its members (ADR 0214), and calls each member's
method:

```rust
#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Clone, Copy)]
pub enum Observer {
    Intersection(&'static IntersectionObserver),
    Resize(&'static ResizeObserver),
}

match observer {
    Observer::Intersection(observer) => observer.observe(child),
    Observer::Resize(observer) => observer.observe(child, None),
}
```

Rust's types make the two arms two: each `observe` is its own type's,
and a `ResizeObserver`'s takes options. Both are the same JS,
`observer.observe(child)`, so the output was

```js
if (observer instanceof IntersectionObserver) {
  observer.observe(child);
} else {
  observer.observe(child);
}
```

The port's draft cast each observer to an `IntersectionObserver`, `unsafe
js::cast`, to call one `observe`: a workaround AGENTS.md forbids.

## Decision

**An `if` whose two branches are alike whole, each statement written alike
(`decisions::same_stmt`), of a test that does nothing (`has_effects`), is
its branch** (tidy's `chains` stage, `alike branches`, `decisions.rs`'s
`alike_whole`), after `written elses` makes a `match`'s last arm an
`else`:

```js
observer.observe(child);
```

- **A test that does something stays**: `if (ready()) { f(); } else {
  f(); }` keeps it.
- **Branches alike in part stay**: arms of which one does more,
  `observe(child)` and `observe(child); disconnect()`, keep the test.
- **The rule is conditional's, for statements.** `test ? a : a` of a test
  that does nothing is already `a` (`logicals.rs`). A program writes no
  other such `if`: clippy's `if_same_then_else` warns of one, and a
  `match`'s arms of one type are one arm, `A(o) | B(o)`. So only the arms
  of a `match` that Rust's types keep apart meet it.

## Why

- **It's upstream's JS**, `observer.observe(child)`, and it's TypeScript's
  rule: a member common to each type of a union is called on the union.
- **No binding or trait of the union is needed.** The alternative was a
  generated trait per group of classes with a method of the same name,
  `ObserverExt`. That's a second form of a union beside the enum (ADR
  0214), and it needs a naming rule and a list of groups no WebIDL has.

## Alternatives

- **A trait in webapi for each set of classes that share a method**,
  `&dyn Observer`: rejected above.
- **The `match` lowered to one call where each arm's JS is alike**, in MIR
  lowering: the arms are separate blocks there, and they're alike only as
  JS, after lowering. Tidy compares the statements it has.

## What holds it

- `test/lowering-switches.test.ts`, "a match of an untagged enum whose arms
  are alike in JS is the one call": the observers' `observe` and
  `disconnect` are one call each, a test that does something stays, and
  arms alike in part keep their `instanceof`; run, with no
  `IntersectionObserver` global, which the old test would throw on.
- Mutations `alike-whole-kept`, `alike-whole-test-dropped` and
  `alike-whole-first-statement` (`lower/mir/tidy/decisions.ts`).
