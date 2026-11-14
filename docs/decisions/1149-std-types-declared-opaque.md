# 1149. std's representations a crate exports are declared opaque, by `@rust-js/std`'s types

Status: Accepted (2026-11-14, #1752, the form the lead approved).

Case ([0262](0262-when-rust-and-js-disagree.md)): none. Only declarations
change; the JS is what it was.

## Context

A crate that exports a `LazyLock` or a `LazyCell`, `pub static ORIGIN:
LazyLock<Point> = LazyLock::new(|| ..)`, gives JS the cell itself, `{ init
}` ([ADR 0318](0318-lazy-cells.md)), whose value is made the first time a
Rust read forces it, by a runtime helper no caller imports. Its
declaration was `export const ORIGIN: any;`: TypeScript took anything for
it, a `LazyLock<number[]>` given where a `LazyLock<Point>` is read too.

Neither of the two other forms is right. `T`, the value, isn't what the JS
gives. The cell's own shape, `{ init?: (() => T) | null; value?: T }`,
would make a representation of rust-js's an API, which a consumer could
read before it's made, or write, or make by hand.

## Decision

Such a type is `@rust-js/std`'s, opaque: `LazyLock<T>` and `LazyCell<T>`,
each an interface branded by a `unique symbol` of its own, which TypeScript
can't make one of, only take of what rust-js's JS gave and give back,
checking what it holds:

```ts
import type { LazyLock } from "@rust-js/std";
export const ORIGIN: LazyLock<Point>;
export function xOf(point: LazyLock<Point>): number;
```

One catalog, `src/lower/declarations/std_types.rs`'s `OPAQUE`, says which
std types are declared so: a crate's declarations name each by it, and
`rust-js --std-declarations` writes `@rust-js/std`'s `index.d.ts` of it,
which `bun run std` commits beside `runtime.js` and the package names as
its `types`. No second list: the committed file is checked to be the
catalog's output, and TypeScript, strict, checks it. Duration and the
other std types declared `any` extend the same catalog as they come.

`std_types/lazy.rs` is the one owner of the cell's object, its `INIT`,
`lazy_of` and `is_made`, which its lowering, its `Debug`, its display and
its `Default` read, where each wrote `"init"` of its own; the runtime's
`$force` and `$forceLock`, which JS can't share it with, read it as
written there.

## Consequences

- A crate's `.d.ts` imports `@rust-js/std`'s types where it names one, as
  its JS imports the package's helpers.
- TypeScript refuses a `LazyLock<number[]>` given for a `LazyLock<Point>`,
  and a cell made by hand, `{ init: () => .. }`.
- Held by `test/declarations-lazy.test.ts`, strict TypeScript callers of a
  `LazyLock` and a `LazyCell` taken, given and given back, and the JS run;
  `test/std-package.test.ts`'s "the committed @rust-js/std's types are the
  catalog's"; mutations `std-opaque-undeclared`, `std-opaque-unbranded`
  and `std-opaque-of-its-item-alone`.
