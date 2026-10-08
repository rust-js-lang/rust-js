# 0048. Let chains: each part runs only once the ones before it held

Status: Accepted. Extends [0030](0030-option.md), whose `if let` this chains.

Case: C, A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

Rust 2024 joins `let`s and conditions with `&&`:

```rust
if let Some(h) = half(n) && h > 2 { h } else { -1 }
```

rustc gives an `if` whose condition is `&&`s with `Let`s among them, which
rust-js rejected. The playground wrote them as nested `if`s (ADR 0044).

Two rules make a chain more than an `&&`:

- **A part may read what an earlier `let` bound** (`h > 2` reads `h`), so
  the bindings must exist before it.
- **A part runs only if every part before it held.** A later `let`'s value,
  `let Some(q) = f(h)`, must not be computed after an earlier part failed.
  The `else` runs if any part fails.

## Decision

**Parts that need no statements of their own join one test.** A `let`
whose value is already a variable, or goes in a `const` before the test,
and conditions that are plain expressions, are all one `if`:

```js
const h = half(n);
if (h != null && h > 2) {
  return h;
} else {
  return -1;
}
```

**A part that needs statements opens an `if` inside**: a later `let` of a
call, whose `const` must come after the tests before it. With an `else` of
one statement, a `throw` or an assignment, each `if` has it, as a person
writes it. (Amended: it was a labeled block's, as a longer one is.)

```js
const h = half(n);
if (h != null && h !== 0) {
  const q = counted(h);
  if (q != null && q > 1) {
    v = q;
  } else {
    v = 0;
  }
} else {
  v = 0;
}
```

With a longer `else`, the `if`s are in a labeled block, the `else` after
them, and the `then` leaves the block when it's done:

```js
chain: {
  const h = half(n);
  if (h != null) {
    const q = counted(h);
    if (q != null && q > 1) {
      return q;
    }
  }
  calls.value = (calls.value + 10) | 0;
  return -calls.value | 0;
}
```

- **Without an `else`**, the `if`s just nest.
- **`while let` chains** come out as `while let` does. Rust writes a `while`
  as `loop { if .. else { break } }`, so the chain is that `if`'s.
- **`&x` is tested where `x` is**, in any `if let` or `match`: a reference
  is the value (ADR 0023), and `x` can't change while it's borrowed. So
  `if let Some(submit) = &on_submit` is `if (onSubmit != null ..)`, with no
  `const`. So is what a binding gives back as it is, `kind_of(&children)`
  or `classify(value)`: `if (isValidElement(children))`, where it was `const
  element = children`, whose `const` opened a level of its own. (Amended.)
- **A `typeof` of a primitive after `!= null` of the same variable is the
  `typeof` alone**, which holds of no `null`: `Some(inner)` then
  `Kind::String(text)` of it is `typeof inner === "string"`. (Amended.)

## Why

- **The common chain is one `if`,** as hand-written JS has it. Nesting only
  appears where Rust's order needs it, when a later part has to wait.
- **A labeled block keeps a long `else` once.** Copying it into each level
  would repeat code, and a flag (`let matched = false`) is more lines for
  the same thing. One statement, a `throw`, is what a person repeats
  rather than label, as react.dev's TerminalBlock's JS now reads.

## Alternatives

- **One `&&` of everything**, with each `let`'s value computed first.
  Wrong: a later `let` would run even when an earlier part failed.
- **Assignments inside the test**, `(q = f(h)) != null`. That's one `if`,
  but it declares `q` before the chain with `let`, and assignment in a
  condition is what JS code is usually told to avoid.

## Consequences

- A chain with more than one level and an `else` of several statements
  uses a labeled block, which JS code rarely has. It's only there when a
  later `let` has to wait for what comes before it.
- An `else` of one statement is written once for each level.
