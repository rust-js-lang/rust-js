# 1008. What a package's entry re-exports is imported by the package's name

Status: Accepted. The user's decision. Amended (the user's decisions):
the default entry alone; the Rust's path decides; a generated root too.
Extends
[0414](0414-a-library-on-npm-as-rescripts-are.md), a library's crate as an
npm package, and [0100](0100-separate-crates.md), a library's contract.
React port gap rd55.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. Where every file
the package's root may be re-exports the module's binding, the package's
name and the module's file are one binding of one module instance; where
any one may be another, or runs code of its own as it loads, the file is
imported, as before.

## Context

React DOM's `ReactDOMOption` writes `import {Children} from 'react'`.
Ported, `use react_isomorphic::react_client::CHILDREN;` printed `import
{Children} from '../../../react/src/ReactClient.js'`: a library's item was
imported only by its module's file, from the consumer's (ADR 0100), or by
the package's name and that file, `@acme/greet/src/lib.js`, of a package
in `node_modules` (ADR 0414). React's `packages/react/index.js`, the
package's `main`, says `export {Children, ..} from './src/ReactClient'`,
but rust-js read no package's entry. And it couldn't: upstream's is Flow,
`export type Element<+C> = React$Element<C>;`, which oxc doesn't parse.

The user decided: the port's package entries keep only plain JS
re-exports in `index.js`, their Flow types moved to an `index.d.ts`, as
ADR 1005 moved type-only modules; rust-js reads the entry with oxc and
imports an item the entry re-exports by the package's name.

## Decision

**A library whose crate's directory has a `package.json` with a `name`
says, in its contract, what its root re-exports of its modules; a
consumer imports such an item by the package's name and the root's name
for it, `import { Children } from "react"`, where Node finds that package
from the consumer's directory.** One owner, `src/packages.rs`, reads a
crate's package and finds where Node finds one; `to_oxc::entry_exports`
parses an entry.

- **The root's entries**: each file `exports["."]` may be, under every
  condition, nested ones and fallbacks too; without `exports`, `main`,
  else `index.js`, as Node finds it. An item is the root's only where
  every one of them re-exports it, by one name, from its own module: a
  host's condition then can't give another module. React's own `".":
  {"react-server": "./react.react-server.js", "default": "./index.js"}`
  is one of these: its `react-server` entry re-exports `ReactServer`'s
  `Children`, so `ReactClient`'s is imported by its file.
- **What an entry re-exports**: `export { a as b } from './m'` and
  `export * from './m'`, of a specifier naming one of the crate's
  modules' JS, where it's written or in source, with or without its
  extension or as its directory's `index`. An item is its module's own,
  keyed by where it's defined: one that module re-exports of another is
  imported by its own module's file, as before. An `export *` names each
  of the module's exports but `default` that nothing else in the entry
  exports, and none where two modules' do, or another `export *`'s
  specifier isn't of the crate's modules, as JS then exports neither.
- **Re-exports alone.** An entry with any other statement, code that runs
  as it loads, which the Rust doesn't, gives no item of the root: each is
  imported by its file.
- **Refused, not passed over**: an entry oxc can't read as JS, Flow's
  types in it, fails the library's build, naming it and saying to keep
  only JS in it and its types in a `.d.ts` beside it.
- **None of a package whose root is one of the crate's own modules' JS**,
  ADR 0414's `"main": "src/lib.js"`: its consumers import it by
  `@acme/greet/src/lib.js`, as before. Nor of a crate compiled alone,
  whose `Cargo.toml` above it may be another's.
- **Where Node finds it.** A consumer imports by the package's name only
  where the first `node_modules/<name>` from its crate's directory up is
  the library's package directory, through its links, a workspace's
  `node_modules/react` of `packages/react`. One not installed, or another
  copy, is imported by the file.
- **Its inputs**: the `package.json` and each entry read are of the
  library's `inputs`, fingerprinted, which a consumer checks ("dependency
  artifact changed"), and of what Cargo is told the build read, so an
  edit of either builds the library again, then its consumers. The
  contract's `package` is optional: a library written before has none,
  and a compiler before this one is another build, whose manifests are
  refused (ADR 1003), so `library::VERSION` stays.
- **Declarations** import a library's types by its module's `.d.ts`, as
  before: a hand-written entry's `index.d.ts` says what it says.

## Why

- **It's the user's decision**, and it's what upstream writes: a package
  is imported by its name, as a person writes it.
- **One binding, one module instance**: an entry's `export .. from` is the
  module's own binding, so the name and the file are the same value, and
  what's imported of each is one module's state.
- **Every condition**: which condition a host resolves is unknown to
  rust-js, and a condition's entry of another module is another value.

## Alternatives

- **The crate's root writes the entry**, `pub use react_client::CHILDREN
  as Children;`: the user decided for reading upstream's `index.js`.
- **The default condition alone**: React's `react-server` entry would be
  read as the client's, a different value under a host of that condition.
  Left to the user (below).
- **Stripping Flow from the entry**: a second parser of a language oxc
  doesn't read.

## Consequences

- A library crate with a Flow entry fails to build until its types move
  to a `.d.ts`: React's port's `packages/react/index.js` and
  `react-reconciler/index.js`.
- React's `Children` is still imported by its file, as `react`'s
  `react-server` condition gives `ReactServer`'s: whether a condition's
  entry is the host's to choose, the default's read alone, is the user's
  decision.

## What holds it

- `test/libraries.test.ts`, "an item a package's entry re-exports is
  imported by the package's name": a workspace's library and its
  consumer, `import { bump, hook } from "@acme/kids"` of a named and an
  `export *` re-export, `count` of the same module by its file, one
  module's state (`27`); a condition's entry that doesn't re-export
  `bump`; an entry that logs; the package not installed; and a Flow
  entry refused.
- Mutations `package-entry-conditions-one`, `package-entry-unparsed-kept`,
  `package-entry-star-unread`, `package-entry-not-plain-read`
  (`packages.rs`), `package-imported-wherever-installed` (`library.rs`),
  `package-entry-unread-by-cargo` (`output.rs`) and
  `entry-statements-plain` (`to_oxc/loads.rs`).

## Amendment: the default entry, the Rust's path, and the root module

The user decided rd55's three questions (2026-10-28).

Case ([0262](0262-when-rust-and-js-disagree.md)): B for the first, an
exception listed in ADR 0262's Consequences and the guide's differences
(`semantics.md`, 31); none for the others.

- **The default entry alone.** Of `exports["."]`'s conditions, the one an
  `import` resolves to by default is read: `import` or `default`, and of
  both, only where they agree, as Node takes whichever the object names
  first, an order `serde_json` doesn't keep; a fallback array's first. React's
  `react-server` entry, which gives ReactServer's `Children`, is the
  host's to choose, as it is of upstream's `import {Children} from
  'react'`. Mutation `package-entry-conditions-one` goes: there's one
  entry; `package-entry-default-unread` holds it.
- **The Rust's path decides.** An item of another module than the root is
  imported by the package's name only where the crate's Rust names it
  through the library's root, `use react_isomorphic::CHILDREN;` of the
  root's `pub use react_client::CHILDREN;`: a path whose last but one
  segment is the library's root (`Foreign::import_key`, of the paths of
  each module's HIR, `use`s and expressions). One named by its
  module's path, `react_reconciler::react_fiber_reconciler::..`, is
  imported by its module's file, as upstream's `'react-reconciler/src/..'`
  is by its path. Each module as its own paths say (amended again, the
  user's decision): one module's `kids::bump` is `import { bump } from
  "@acme/kids"`, another's `kids::client::bump` `import { bump } from
  "../../kids/src/client.js"` in its file, one binding of one module
  instance. Mutations `package-root-path-unread` and
  `package-root-path-of-the-crate`.
- **A generated root too: one way.** Where the package's root is the
  crate's root module's JS, ADR 0414's `"main": "src/lib.js"`, what that
  module exports, its own items and its `pub use`s, is what the root
  re-exports, and is imported by the package's name: `import { Greeting,
  greet } from "@acme/greet"`, where it was `@acme/greet/src/lib.js`. A
  root module's item is imported so whatever path names it, as each path
  to it is through the root; its declared types and held bindings too.
  Another module's item is imported by its file, `@acme/greet/src/loud.js`,
  which the host's `exports` check still holds. `test/libraries.test.ts`'
  ADR 0414 tests say `@acme/greet`; its `exports` test refuses a hidden
  `./src/loud.js`. Mutation `package-root-module-by-file`.

## Amendment: a root that re-exports what the entry does writes nothing

The user's decision (2026-10-28). Case: none; what JS imports is the
entry, which already exports each.

React's `react.rs`, to be named through, `pub use react_client::CHILDREN;`,
wrote `packages/react/react.js`, `export {Children} from
'./src/ReactClient.js'`, beside upstream's `index.js`: a second entry.
**A crate's root whose only job is re-exporting what its package's
hand-written entry re-exports writes no JS and no `.d.ts` of its own**:
every item it holds but its `mod`s, its own `use`s and `extern crate`s is
a `pub use` its file re-exports (`reexports::reexports_alone`), and each
of those is of one of the crate's modules, by the name the entry gives
it, from the same module (`output::entry_is_root`, `packages::reexports_each`). A root with any
other item, a type's too, or a `pub use` the entry doesn't re-export so,
writes its JS as before; so does one whose package's root is its own
JS, ADR 0414's `src/lib.js`. Held by `test/libraries.test.ts`' entry
test (no `kids/src/lib.js`; one written where the entry runs code of its
own, and with a `pub type`) and mutations `entry-root-written`
(`output.rs`), `entry-root-unmatched` (`packages.rs`) and
`reexports-alone-of-anything` (`lower/reexports.rs`).
