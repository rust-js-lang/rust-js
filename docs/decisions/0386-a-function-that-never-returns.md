# 0386. A function that never returns, `-> !`

Status: Accepted.

Case: A ([0262](0262-when-rust-and-js-disagree.md)): a value of `!` is
never made, so no program can see how it's held.

## Context

A function that ends the program or loops forever says so with `!`:

```rust
fn fail(message: &str) -> ! {
    panic!("{message}");
}
```

rust-js refused any value of type `!`, its result too, and so the function
whatever it did. rustc's suite had four tests that failed only for that
(`return/ret-bang.rs`, `for-loop-while/loop-diverges.rs`,
`for-loop-while/loop-no-reinit-needed-post-bot.rs`,
`binding/nested-matchs.rs`).

## Decision

**`!` is a type rust-js represents: a function of it is a JS function that
throws or doesn't end, called as any other, and declared as returning
TypeScript's `never`.**

```js
function fail(message) {
  throw new Error(message);
}
```

```ts
export function fail(message: string): never;
```

- **No value of it is ever made**, so nothing holds one: MIR doesn't
  assign it, and what follows a call of one isn't reached.
- **`never` is TypeScript's own word for it**: a caller's code after the
  call is unreachable to TypeScript too.
- **A type with a field of it is represented as any other**: std's
  `PoisonError` has a `_never: !`, which made a `TryLockError` refused but
  for a case of its own in what rust-js represents. That case is gone.

## Why

- **It's how JS writes it**: a function that throws is a plain function.
- **It's tested**: the corpus's `never_returns.rs` calls one in an `if` and
  a `let .. else`, one that loops forever, and panics through one; the declarations
  test declares one `never`; and the four rustc tests pass.

## Amendment: `Infallible` is `!` since Rust 1.100 (2026-11-13)

Case: A, as before: no value of `!` is made, so no program sees how it's
declared, and the declaration says exactly the values there are.

Rust 1.100 made `std::convert::Infallible` an alias, `pub type Infallible
= !;`, where it was std's `enum Infallible {}`. Its declaration was `any`
before, not a decision but the fallback for another crate's type
(`declarations.rs`, "Any other is `any`"); now it's `!`'s, `never`, as
the crate's own enum of no variants, chrono's `enum Void {}`, already was.
So `!` is `never` in every position, a value's as a result's:

```rust
pub fn set_current_update_priority(new_priority: EventPriority, _unused: Option<Infallible>) {}
```

```ts
export function setCurrentUpdatePriority(newPriority: EventPriority, unused?: never | null | undefined): void;
```

- **It's what the value can be**: an `Option<Infallible>` is only ever
  `None`, so `never | null | undefined`, which is `null | undefined`, is
  exact. `any` was looser than the value, a port gap (AGENTS.md).
- **It's what upstream writes**: React's port declares two such
  parameters and fields, which upstream writes as Flow's `empty`,
  `IntentionallyUnusedArgument?: empty` (ReactDOMUpdatePriority.js), and
  `null` (`reason: null`, ReactFlightClient.js). A union that held one,
  `ReactPromise`'s `value`, is now its other members' union, `unknown`,
  where `any` absorbed them.
- **It's tested**: test/declarations-never.test.ts declares an
  `Option<Infallible>` parameter and field and an `Infallible` parameter,
  and TypeScript takes `null` and `undefined` for one and refuses `0`.
