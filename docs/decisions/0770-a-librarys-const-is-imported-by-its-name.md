# 0770. A library's `const` is imported by its name, as its crate reads it

Status: Accepted. Amended (the user's decision): a dependency's module
is imported by its package's subpath where its `exports` resolves it.
Amended: so is a crate's own module whose JS is in another package.
Amends [0031](0031-consts.md), whose `const` of
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
  import without its extension. Amended below: it is, where the package's
  `exports` resolves that subpath.

## What holds it

- `test/cross-crate-items.test.ts`, "a library's const is imported by its
  name": a `SymbolFor`, a number and a string, read in the library and in
  a crate that uses it, run.
- Mutation `library-const-inlined` (`lower.rs`).

## Amendment: a dependency's module by its package's subpath

The user's decision (2026-11-01), React port gap rd349. Case
([0262](0262-when-rust-and-js-disagree.md)): none: where the package's
`exports` resolves the subpath to the module's JS, the specifier and the
file are one module, one instance; elsewhere the file is imported, as
before.

Every ported React module imported a workspace package's module by its
file, `import {enableScopeAPI} from '../../../shared/ReactFeatureFlags.js'`,
where upstream writes `from 'shared/ReactFeatureFlags'`: 1,003 imports in
252 files. Node's ESM resolves a bare subpath without its extension only
through the package's `exports`; React's jest and rollup resolve one by
their own rules.

**A dependency crate's module is imported by its package's name and the
module's path in the package without its extension,
`'shared/ReactFeatureFlags'`, only where that package's `package.json`
`exports` resolves that subpath to the module's JS, where it goes in
source, and Node finds the package from the consumer, as ADR 1008's root
is.** Elsewhere the relative `.js` path stays. rust-js reads `exports` and
never writes it: what a package exports is its author's, the host's
authority.

- **One owner**, `src/packages.rs`, which reads `exports["."]` (ADR 1008):
  `subpaths` resolves each module's subpath as Node's
  `PACKAGE_EXPORTS_RESOLVE` does, by its own key, else the pattern whose
  part before its `*` is longest, then whose key is, under the default
  condition (`default_target`, the same as the root's: `import` or
  `default`, both where they agree). A `null` target, another condition's
  alone, a pattern's target of another file (`"./src/*": "./src/*"`, the
  file without its extension) or a part with a `node_modules` segment,
  which Node refuses, gives none. The library's manifest says each
  (`Package::subpaths`); `library.rs` asks `subpath_specifier`.
- **Inputs**: the `package.json` is already of the library's inputs (ADR
  1008), so an edit of its `exports` builds the library again, then its
  consumers.
- **Declarations** too: a consumer's `.d.ts` imports a type by the same
  specifier, which TypeScript resolves through `exports` to the `.js`,
  then the `.d.ts` beside it.
- **A library in `node_modules`** (ADR 0414): a module whose subpath its
  `exports` resolves is imported by it, `@acme/greet/src/loud`; another is
  relinked by the host, as before, to `@acme/greet/src/loud.js`, where
  `exports` exports that file.
- **A crate's own modules** are imported by their relative `.js` path, as
  before: upstream's `'./ReactDOMComponentTree'` without its extension is
  not this decision.

What a workspace adds for this: `"exports": {"./*": "./*.js"}` beside
any entries its package has (`".": "./index.js"`, `"./jsx-runtime"`),
which Node's ESM, Vite and jest all resolve. A hand-written import of a
subpath with its extension, `'shared/ReactFeatureFlags.js'`, then names
`ReactFeatureFlags.js.js`: write it without, as upstream does.

**A bare specifier never moves an import** (the P1 found after it landed;
ADR 0262's "results change, preserve"). An import's place is the order JS
loads its modules in: React's ReactDOMFizzServerNode imports ReactFizzServer
before ReactFizzConfigDOM, and jest's host-config mocks depend on that
order. Imports were ordered by their specifiers' text, so a module's
`shared/..` moved among the packages, before every relative one, and
ReactFizzConfigDOM moved before ReactFizzServer, which broke 103 react-dom
tests. Each import of a module by its package's subpath is ordered by its
file's specifier, the one it had before, `js::Package::ordered_as`, which
`library.rs` records as it chooses the subpath (`Dependencies::ordered_as`).
Only its text changes.

Held by `test/libraries.test.ts`, "a dependency's module is imported by
its package's subpath where its exports resolve it": no `exports`, a
pattern (run by Node), an exact key under the default condition beside
another condition's pattern, a `null`, a longer pattern of another file,
the root alone, a `node_modules` segment and the package not installed,
each against native Rust, and the same order of imports with `exports`
as without, beside a binding's `../../a.js` and `../../t.js`. Mutations
`package-subpath-import-moved` (`to_oxc/header.rs`), `package-subpath-any-target`,
`package-subpath-pattern-shortest`, `package-subpath-exact-key-unread`,
`package-subpath-segments-unchecked` (`packages.rs`) and
`package-subpath-unread` (`library.rs`).

## Amendment: a crate's own module in another package by that package's subpath

React port gap rd434, the user's rd349 decision extended (taken on their
behalf while away, logged). Case ([0262](0262-when-rust-and-js-disagree.md)):
none, as above: the specifier and the file are one module.

React DOM's crate declares a module whose file is in `packages/shared`,
`#[path = "../shared/ReactDOMFragmentRefShared.rs"]`, as upstream keeps
the file shared with Fabric there. Its JS goes beside it, in `shared`'s
package, and the crate's other modules imported it by its file,
`'../../../shared/ReactDOMFragmentRefShared.js'`, where upstream writes
`'shared/ReactDOMFragmentRefShared'`. "A crate's own modules are imported
by their relative `.js` path" held only of a module in the importer's
own package.

**A crate's module whose JS goes in source in another package than the
importing module's, the `package.json` nearest above each, as Node's
`LOOKUP_PACKAGE_SCOPE` finds it, is imported by that package's name and
subpath where its `exports` resolves the subpath to the module's JS and
Node finds the package from the importing module's, as a dependency's
module is.** One in the importer's own package, or where either fails, is
imported by its file, as before; so is each of a file compiled alone,
whose JS no host writes in source.

- **One owner**: `packages.rs` answers which package a file is of
  (`packages::scope`), with the same subpath resolution as a dependency's
  (`subpath`, shared with `subpaths`); `output.rs` asks it of each module
  where its JS goes in source and chooses the specifier
  (`scope_specifier`), for its JS, its `.d.ts` and an `import()` alike.
- **Inputs**: each other package's `package.json` is of what the build
  read, so an edit of its `exports` builds the crate again.
- **Order**: as a dependency's, by its file's specifier,
  `js::Import::ordered_as`, which `to_oxc/header.rs` orders a crate's
  import by, as it does a package's.
- **The host's relinking** (ADR 0101) leaves a bare specifier as it is:
  it rewrites only the relative ones the manifest's links name.

Held by `test/libraries.test.ts`, "a crate's module in another package is
imported by that package's subpath": no copy installed, no `exports`, a
pattern (run by Node against native Rust), the `.d.ts`'s import, the
module in `shared` importing the crate's other module by its file, a
pattern of another file, and the import's place beside a binding's
`../../a.js`. Mutations `own-subpath-same-package`,
`own-subpath-not-installed` (`output.rs`), `own-subpath-ordered-by-text`
(`to_oxc/header.rs`) and `scope-past-nearest` (`packages.rs`).

## Its imports' order replaced (rd570, ADR 1063)

The order above, a subpath's import kept where its file's specifier put
it (`js::Package::ordered_as`, `js::Import::ordered_as`,
`Dependencies::ordered_as`, and the mutations `package-subpath-import-moved`
and `own-subpath-ordered-by-text`), is replaced by
[1063](1063-imports-are-in-the-order-of-their-uses.md): every import is
where its `use` is, a subpath's too, and those keys are gone. The
specifiers this ADR chooses are as they were.

## Amendment: a subpath with its extension, where `exports` keeps it (rd713)

react-markup's crate root mounts the markup Flight client config in
react-client's directory, `#[path =
"../react-client/src/forks/ReactFlightClientConfig.markup.rs"]`, and that
module re-exports react-markup's `ReactMarkupLegacyClientStreamConfig`.
Upstream writes `'react-markup/src/ReactMarkupLegacyClientStreamConfig.js'`;
rust-js wrote `'../../../react-markup/src/..'`, climbing out of
react-client's package into react-markup's. The module is in another
package, and Node finds `react-markup` from it, so the rule above applies,
but react-markup's `exports` is `"./src/*": "./src/*"`, which resolves the
subpath with its extension, and `packages::subpath` asked only for the one
without it, `./src/ReactMarkupLegacyClientStreamConfig`, which that
pattern resolves to no file. **A package's subpath to a file is its path
there without its extension where `exports` resolves that to the file,
else with it**, as Node resolves each. Case N of ADR 0262: the same file,
which Node finds by that specifier. Held by `test/libraries.test.ts`'s two
subpath tests (`"./*": "./*"` and `"./src/*": "./src/*"` each import by the
subpath with its extension, run in Node); mutation
`package-subpath-extension-untried` (`packages.ts`); guide
project/dependencies.md.
