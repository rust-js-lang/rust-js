# 1008. What a package's entry re-exports is imported by the package's name

Status: Accepted. The user's decision. Extends
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
