# 0546. A declaration names a function as its JS does

Status: Accepted. Extends [0196](0196-typescript-declarations.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): what TypeScript reads
of a module is what it exports.

## Context

A function named as JS keeps a word apart, itertools' `chunk_by::new`, is
exported as `new$`, its name made apart from the module's others and JS's
words where the crate's items are named. Its declaration took Rust's name
again, `export function new<K, J, F>(..)`, which TypeScript can't read:
"rust-js can't read the declarations it wrote".

## Decision

**A function's or a `const`'s declaration takes its name from the one
place the JS's is answered, the crate's `FnInfo`, `new$`.** The
declarations are given the crate's names, as a module's JS is.

## Why

`declarations.test.ts` declares a library's `pub fn new()` as `export
function new$(): number;`, which its JS exports, and type-checks a
TypeScript import of it.

## Consequences

itertools' declarations are written. A mutation names it as Rust does.
