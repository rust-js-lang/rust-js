# 0770. A library's `const` is imported by its name, as its crate reads it

Status: Accepted. Amends [0031](0031-consts.md), whose `const` of
another crate was its value, written in place, and
[0100](0100-separate-crates.md), whose consumer imported a library's
functions alone.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: a `const` is the
same value read by its name or written in place, each use a value of its
own; only how the JS reads changes.

## Context

React's `shared/ReactSymbols.rs` has `pub const REACT_MEMO_TYPE: SymbolFor
= SymbolFor("react.memo")`. Inside `shared`, a read of it was its name,
imported from its module; in the react crate, `ReactMemo`'s
`r#typeof: REACT_MEMO_TYPE` was `$$typeof: Symbol.for('react.memo')`, and
no import. Upstream writes `import {REACT_MEMO_TYPE} from
'shared/ReactSymbols'`. The library's manifest exports the `const` (ADR
0100), as it does a function; `named_const` asked whether the crate itself
had the item, `krate.fns`, where a function's call asked `is_rust_fn`,
the crate's or a library's: two answers to whether an item rust-js
compiled is read by its name.

## Decision

**A `const` or `static` rust-js compiled, the crate's or a library's, is
read by its name, imported from the module that declares it**, as a
function is: `is_rust_fn` answers it for each. A use that may change it
is a copy, in a crate as across one (ADR 0031). Anyone else's, `u32::MAX`
or a binding crate's, is its value, as before.

```rust
// shared
pub const REACT_MEMO_TYPE: SymbolFor = SymbolFor("react.memo");

// react
let element_type = Memo { r#typeof: REACT_MEMO_TYPE, r#type, compare };
```

```js
import {REACT_MEMO_TYPE} from '../../shared/ReactSymbols.js';

const elementType = {$$typeof: REACT_MEMO_TYPE, type, compare: compare ?? null};
```

## Why

- **One answer**: whether an item is read by its name is whether rust-js
  compiled it, and has its name, whichever crate it's in.
- **Upstream's JS**: React imports each symbol by its name; a value
  written in place loses the name the reader looks for.

## Alternatives

- **A library's `const` written in place, its own read by its name**: a
  `const` moved to another crate changes every read of it.

## Consequences

- The import is of the library's module by its path, as a function's is:
  `../../shared/ReactSymbols.js`, not upstream's `shared/ReactSymbols`.
  A workspace crate's directory that is an npm package of the workspace,
  React's `packages/shared`, is not yet imported by the package's name, as
  a crate an npm package has is (ADR 0414); nor does rust-js write an
  import without its extension.

## What holds it

- `test/cross-crate-items.test.ts`, "a library's const is imported by its
  name": a `SymbolFor`, a number and a string, read in the library and in
  a crate that uses it, run.
- Mutation `library-const-inlined` (`lower.rs`).
