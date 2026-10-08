# 0283. The js crate covers TypeScript's ES library, and a test holds it

Status: Accepted. Extends [0102](0102-js-and-webapi.md) and [0281](0281-webapi-covers-typescripts-dom.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

[ADR 0281](0281-webapi-covers-typescripts-dom.md) holds the `webapi` crate
to TypeScript's DOM. The js crate, JS's own library, had what ports had
needed: JSON, a regular expression's `test`, a `Uint8Array`'s length. It
had no `Date`, no `Intl`, no typed array but bytes, no `WeakMap`, no
`Symbol`: what ReScript's Stdlib and TypeScript's `lib.es*.d.ts` have.

Most of JS's library is Rust's already. `Vec` is an `Array`, `str` a
string, `f64` a number with `Math`'s functions, `HashMap` a `Map`
(ADRs 0034, 0036, 0059). What's left is what Rust has no type for.

## Decision

**The js crate's goal is every member of the built-ins Rust has no type of
its own for, as TypeScript's ES2024 libs declare them; a test measures it.**

- **The built-ins measured** (`builtins/coverage.ts`'s `SCOPE`): `Date`,
  `RegExp`, `Error`, `Promise`, `Symbol`, `ArrayBuffer`,
  `SharedArrayBuffer`, `DataView`, the eleven typed arrays, `WeakMap`,
  `WeakSet`, `WeakRef`, `FinalizationRegistry`, `Proxy`, `Reflect`,
  `Atomics`, `JSON`, `Intl` and its classes, and the global functions
  (`parseInt`, `encodeURIComponent`, ..). Not `Array`, `String`, `Number`,
  `Math`, `Map`, `Set` nor `Object`: Rust's own types are those.
- **The reference** is TypeScript 5.9's `lib.es5` to `lib.es2024` files,
  pinned as the `typescript-es` package, read with TypeScript's parser: a
  built-in's interface's members, its constructor interface's (`Date.now`
  a static), and `new` where it has one.
- **A ratchet**: `builtins/coverage.txt` lists each member bound;
  `test/builtins-coverage.test.ts` fails when one isn't any more, or a new
  one isn't blessed, as webapi's does.
- **The bindings are written, not generated** (ADR 0116), as ReScript's
  Stdlib is: a built-in is a type, its members its methods (`date.get_time()`),
  its constructors and statics a module of its name (`date::now()`), as
  webapi's are (ADR 0282). Rust's own types' JS functions, `string::trim`,
  stay functions: Rust's `str` can't have methods of another crate's.

## Why

- **The same promise as the DOM's**, measured the same way.
- **Only what's missing is counted**: a program reaches `Math.sin` as
  `x.sin()`; measuring `Math` would count what Rust has as missing.

## Alternatives

- **Generate them from the `.d.ts`**: rejected for bindings (ADRs 0116,
  0119). TypeScript's overloads and generics have no one Rust a generator
  can choose; `Date` and `Intl` are few enough to write.
- **Measure all of JS's library**: a number that says `Array.prototype.at`
  is missing where `v.get(i)` is it.

## Consequences

- `typescript-es`, TypeScript 5.9.3, is a dev dependency beside TypeScript 7,
  which ships no lib files.
- At first 14 of 691 members, 2.0%.
