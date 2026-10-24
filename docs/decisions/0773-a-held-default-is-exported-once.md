# 0773. A binding a module's default names is exported as the default alone

Status: Accepted; amended (every importer asks what a module exports an
item as, below). Amends [0633](0633-a-binding-named-through-a-module-is-read-through-it.md),
whose `pub use` of a binding the default names was exported by its name
as well, and [0625](0625-a-default-constant-is-its-value.md), whose
constant a `pub use` of its module renames is declared.

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

## Amended: every importer asks what a module exports an item as

Case ([0262](0262-when-rust-and-js-disagree.md)): R: the results
changed. React's `shared/noop.rs` is `js::export_default! { pub fn
noop() {} }`, `export default function noop() {}` and nothing else (ADR
0611, amended). Its crate imported it as its default (ADR 0251), but the
library's manifest listed it by its name, so the react crate's ReactLazy,
`use shared::noop::noop;`, was `import { noop } from
'../../shared/noop.js'`, and its call threw `TypeError: noop is not a
function`, where Rust calls it; the binding's default above was right, as
the manifest asked `HeldBinding::import`. A `pub use` of a default in its
crate, `export { noop } from "./noop.js"`, another module's default
naming it, `export { noop as default } from`, and a block form's `const`
read so were wrong the same way: each decided the import form on its own.

**What a module exports an item of its own as, its default or its name,
is `default_exports::exported_as`'s, of either form of
`js::export_default!`, and every importer asks it**: a module of its
crate (ADR 0251), another crate through the library's manifest, whose
item says `default` (`Item::default`), a `pub use` re-exporting it, a
module's default naming it, and a `js::import!` of it (ADR 0304).
`default_exports.rs` owns the macro's forms and this answer; what a
module holds of a binding is `HeldBinding`'s, as above.

```rust
// shared/noop.rs
js::export_default! {
    pub fn noop() {}
}
```

```js
// ReactLazy.js
import noop from '../../shared/noop.js';
```

```js
// shared/index.js, `pub use crate::noop::noop;`
export { default as noop } from "./noop.js";
```

A default of the naming form, `js::export_default!(page);`, a `pub fn`
exported by its name as well (ADR 0611), is imported as the default by
another crate too, as its own crate imports it: one answer, whichever
form made it. A consumer's import is named after the item, `import
VERSION from "../dep/version.js"`, not the file. A constant the default
holds that a `pub use` of its module renames, `pub use self::VERSION as
RELEASE;`, is declared, then exported, as one its module reads is (ADR
0625, amended): `export { VERSION as RELEASE }` reads it by its name.

## Consequences

- The import is of the library's module by its path,
  `../../shared/assign.js`; upstream's `shared/assign`, by its package's
  name, isn't yet written (ADR 0770).

## What holds it

- `test/cargo-link.test.ts`, "a library's module that holds a binding is
  what a consumer reads it through": the library's `assign.js` and
  `.d.ts`, its default alone, and the consumer's `import assign`, run.
- Mutation `held-default-exported-by-name` (`bindings.rs`).
- `test/cross-crate-items.test.ts`, "a library's default export is
  imported as its default": each form's default, a `const`'s, read in its
  crate, re-exported by a `pub use` and a module's default, and imported
  by another crate, run.
- `test/architecture.test.ts`, "what a module exports an item as is
  exported_as's alone".
- Mutations: `default-exported-as-name`, `own-default-of-another-module`
  (`default_exports.rs`), `library-default-listed-by-name`
  (`lower/library.rs`), `library-default-imported-by-name`
  (`library.rs`), `library-default-named-after-file` (`analysis.rs`),
  `aliased-default-undeclared`, `reexported-default-by-name`
  (`pipeline.rs`).
