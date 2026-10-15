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

## Why

- **It's how JS writes it**: a function that throws is a plain function.
- **It's tested**: the corpus's `never_returns.rs` calls one in an `if` and
  a `let .. else`, one that loops forever, and panics through one; the declarations
  test declares one `never`; and the four rustc tests pass.
