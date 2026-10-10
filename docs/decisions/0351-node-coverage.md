# 0351. The node crate is measured against @types/node, and only grows

Status: Accepted. Extends [0346](0346-next-coverage.md)'s and
[0347](0347-react-coverage.md)'s ratchet to [0272](0272-node.md)'s crate.

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

The node crate is typed as `@types/node` types Node.js (ADR 0272), one
binding at a time as a program needs it: `fs`'s `readFileSync`,
`process.cwd`, `http`'s request and response. Nothing said how much of
Node.js that is, and `@types/node` wasn't installed to say.

## Decision

**Each export of each module `@types/node` declares is listed, `+` where
the crate binds it and `-` where it doesn't, and the count only grows.**

- **`@types/node` 24.19.2** is a pinned dev dependency, the latest of
  Node.js 24, the LTS rust-js runs on.
- **The measure**: `node/coverage.ts` reads each `declare module "fs"`,
  `node:fs` as `fs`, and lists its exports, as TypeScript has them: each of
  its declarations, unless it says `export {}`, then those it marks; what
  it re-exports; and of one that's `export =` a value, `path`, `events`,
  that value's members, its interface's, its class's statics, its
  namespace's. `process` and `console`, `export =` the globals, are their
  interfaces' members.
- **What's bound**: a value by its `link_name`, `fs#readFileSync`,
  `process.cwd`; a type or a class by an item of its name.
- **The ratchet**: `node/coverage.txt` is that list, which
  `test/node-coverage.test.ts` holds, as the react and next crates' are.

## Why

- **The same promise as React's and Next.js's**: what's `-` is what's
  left, from `@types/node`'s own list.

## Consequences

- Today: 5 of 1538, `fs` 1 of 171, `http` 3 of 30, `process` 1 of 77.
- A global outside a module, `Buffer`'s, `setTimeout`'s, is counted where
  a module exports it, `buffer`'s, `timers`'.

## Amendment: an item is its file's module's

A type or class item counted by its name alone made `http::Server` count
`https#Server`, `net#Server` and `tls#Server` too. An item now counts for
the module of its file, http.rs's for `http`; lib.rs's, the crate's own
types such as `BufferEncoding`, for each module.

## Amendment: a re-export binds

A type the crate re-exports, `pub use webapi::{URL, ..}` in url.rs, binds
that module's export: Node's `url.URL` is the global webapi binds.

## Amendment: what isn't bound by design

As react's (ADR 0347), what isn't bound by design is `x`, counted apart,
each list with why: so far, TypeScript's own types of types, buffer's
`WithImplicitCoercion` and its kin.
