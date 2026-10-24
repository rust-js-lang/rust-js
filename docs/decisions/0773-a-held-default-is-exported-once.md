# 0773. A binding a module's default names is exported as the default alone

Status: Accepted. Amends [0633](0633-a-binding-named-through-a-module-is-read-through-it.md),
whose `pub use` of a binding the default names was exported by its name
as well.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: no Rust program can
tell; what JS can import of the module changes, to upstream's.

## Context

React's `shared/assign.js` is `const assign = Object.assign; export
default assign;`, and the react crate's `ReactBaseClasses` imports it,
`import assign from 'shared/assign'`. Its port is `use
js::object::assign; js::export_default!(assign);`, which the react crate
can't name: a private `use`. `pub use` lets it, and a crate naming it
already read it through the module's default (ADR 0633); but `pub` made
the module export it by its name too, `export const assign =
Object.assign;`, which no caller read and upstream doesn't have.

## Decision

**A binding a module holds is exported once: as the module's default
where `js::export_default!` names it, else by its name where a `pub use`
names it** (`HeldBinding::exported_by_name`). Every caller reads it as
the default, in its crate and in another; `pub` is how another crate
names it in Rust.

```rust
// shared/assign.rs
pub use js::object::assign;

js::export_default!(assign);
```

```rust
// react: ReactBaseClasses.rs
use shared::assign::assign;
```

```js
// shared/assign.js
const assign = Object.assign;

export default assign;
```

```js
// ReactBaseClasses.js
import assign from '../../shared/assign.js';
```

Its `.d.ts` declares it so too, `declare const assign: typeof
Object.assign; export default assign;`.

## Why

- **What's exported is what's read**: no caller reads it by name.
- **Upstream's module**, line for line.

## Alternatives

- **A form of its own for another crate's default, `use
  shared::assign::default`**: Rust has no name `default` to import; a
  second spelling of what `pub use` already says.
- **Every default the naming form names exported alone, a `pub fn`'s
  too**: a `pub fn` is called by its name, in its crate and in another,
  which is what it's exported as (ADR 0611).

## Consequences

- The import is of the library's module by its path,
  `../../shared/assign.js`; upstream's `shared/assign`, by its package's
  name, isn't yet written (ADR 0770).

## What holds it

- `test/cargo-link.test.ts`, "a library's module that holds a binding is
  what a consumer reads it through": the library's `assign.js` and
  `.d.ts`, its default alone, and the consumer's `import assign`, run.
- Mutation `held-default-exported-by-name` (`bindings.rs`).
