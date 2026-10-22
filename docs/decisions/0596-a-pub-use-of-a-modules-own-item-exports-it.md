# 0596. A `pub use` of a module's own item exports it by that name

Status: Accepted. Extends [0240](0240-reexports.md)'s re-exports and
[0573](0573-every-rust-name-is-camel-case-in-js.md)'s names.

Case ([0262](0262-when-rust-and-js-disagree.md)): A.

## Context

React's scheduler exports its own functions, and its priorities, by the
names its API has:

```js
export {
  ImmediatePriority as unstable_ImmediatePriority,
  ...
  shouldYieldToHost as unstable_shouldYield,
  getCurrentTime as unstable_now,
};
```

In Rust that's `pub use`:

```rust
#[cfg_attr(rust_js, rust_js::name = "unstable_now")]
pub use self::get_current_time as unstable_now;
#[cfg_attr(rust_js, rust_js::name = "unstable_ImmediatePriority")]
pub use scheduler_priorities::IMMEDIATE_PRIORITY as UNSTABLE_IMMEDIATE_PRIORITY;
```

A `pub use` of another module's function was a re-export (ADR 0240); one of
the module's own was dropped, and one of a constant too. And a `pub use ..
as` named the item in the module's own code as well: the scheduler would
have imported `ImmediatePriority` as `UNSTABLE_IMMEDIATE_PRIORITY` and read
it so.

## Decision

**A `pub use` exports its item by the name it gives**, in the JS and the
`.d.ts`, whatever module the item is of:

- another module's function or constant: `export { IMMEDIATE as
  unstable_ImmediatePriority } from "./priorities.js";`, as before for a
  function;
- the module's own function by another name: `export { currentTime as
  unstable_now };`, after its functions, as `export default` is.

The name is the `use`'s `#[rust_js::name]`, as any item's is, or its
name camelCase, `time_now` as `timeNow` (ADR 0573).

**The module reads the item by its own name**, not the name it exports it
by: a `pub use` renames what others import, never what its own code says,
as JS's `export { a as b }` doesn't. A private `use .. as` still renames
the import, `import { client as sandpack }`.

The `.d.ts` model has one kind for both, `{ kind: "export", from?, names }`,
`export { .. }` with or without `from`, as JS has one statement.

## Consequences

- An alias's local is an exported function, so nothing reclaims its name
  (ADR 0352).
- A `pub use` of a type still re-exports nothing (ADR 0240).

## What holds it

- `test/modules.test.ts`, "a pub use by another name of the module's own
  function exports it by that".
- `test/manifest.test.ts`, "a manifest says what each module exports":
  the alias is among them.
- The guide's [Emitting modules](../guide/project/modules.md#re-exports).
- Mutations: the own alias dropped, a constant's re-export dropped, a
  `pub use` renaming its own module's reads, the alias unprinted, not in
  the manifest, not in the `.d.ts`.
