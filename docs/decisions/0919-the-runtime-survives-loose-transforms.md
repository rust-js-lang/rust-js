# 0919. The runtime spreads only arrays, so an app's loose transforms leave its answers as they are

Status: Accepted. Found porting React's reconciler: its useId tests ran the
runtime through React's Jest config. Amends
[0103](0103-runtime-package.md) (what the package's source may be written
with).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results change. Under
Babel's loose spread, `format!("{:0>width$}", digits)`'s `$pad("dalala", 6,
">", "0")` was `"00000dalala"`, so React's useId test read
`_R_100000lalala_` where the server wrote `_R_1lalala_`.

## Context

`@rust-js/runtime` is an app's dependency, and an app's tooling may
transpile its dependencies: React's Jest config runs Babel over everything
it loads, with `@babel/plugin-transform-spread` and
`@babel/plugin-transform-destructuring` in loose mode. Loose spread assumes
every spread value is an array:

| Written | Loose spread | Of a string `"ab"` | Of a `Set` `{1, 2}` |
| --- | --- | --- | --- |
| `[...x]` | `[].concat(x)` | `["ab"]` | `[set]` |
| `f(a, ...x)` | `f.apply(void 0, [a].concat(x))` | one argument | one argument |
| `f(...x)` | `f.apply(void 0, x)` | `TypeError` | no arguments |

The runtime counted a string's chars as `[...text].length`, cut them as
`[...text].slice(..)`, split an empty pattern as `["", ...s, ""]`, and
stepped sets and maps as `[...v]`: 16 array spreads, 9 of them of a
string, a set or a map. A call's spread had a second fault, of its own:
JS takes a bounded number of arguments, so `v.push(...bytes)` of a
megabyte, a `Vec<u8>`'s `write!`, a heap's `append`, a `splice`, a
`repeat`, was a `RangeError` (Node's bound is near 120,000, Bun's higher),
even untranspiled.

Loose destructuring, `const [a, b] = x` as `a = x[0], b = x[1]`, is exact
of an array, and each of the runtime's array patterns takes one: a
helper's pair, a map's entry. Loose for-of steps `x[Symbol.iterator]()`
wherever there is one, exact of a string, a map and a set alike.

## Decision

- **The runtime spreads only what's evidently an array**: an array
  literal, `Array.from(..)`, `Object.keys(..)` and the like, a method only
  an array has (`map`, `filter`, `split`), a rest parameter, or a `const`
  made of one. A string's chars are `Array.from(s)`, a set's or a map's
  items `Array.from(m)`.
- **Many items are added by `$extend(v, items)`**, a loop, never spread
  into one call: a `Vec<u8>`'s writes, `$splice`, `$repeatItems`,
  `$heapAppend`. A call given an array-like's items as its arguments,
  bounded, is `f.apply(null, units)`: `String.fromCharCode`'s, 4,096
  units at a time.
- **`test/runtime-transpiled.test.ts` holds both**: it parses each of
  `src/runtime/*.js` and fails on a spread of anything else, and it runs
  the compiler's runtime through Babel's loose spread, destructuring,
  for-of and template literals, each helper that steps text or a
  collection answering as it does untranspiled.
- **for-of stays.** Babel's loose for-of is exact of every iterable. A
  transform that steps every for-of as an array, Babel's `assumeArray` or
  TypeScript's ES5 output without `downlevelIteration`, isn't one an app
  can use: the JS rust-js writes steps maps and sets with for-of too, as
  `for (const [k, v] of m)`, and those loops would run no times. Writing
  the runtime's string loops as `Array.from(s)` wouldn't make such an app
  run.

## Consequences

- An app may transpile `@rust-js/runtime` with Babel's loose spread and
  destructuring, as React's Jest does, and its answers are the same.
- `runtime_long_writes.rs` writes a megabyte to a `Vec<u8>`, splices,
  repeats and appends a million items, against native Rust.
- The compiler's own output spreads only arrays: `Array.from(..)`, a
  `Vec`, a rest parameter. Its `v.push(...$copyRange(v, a, b))` of
  `extend_from_within`, and a Debug builder's `parts.push(...entries)`,
  have the argument bound: #334.

## Alternatives

- **Spread of a string, refused only where it's written of one**: the
  runtime's parameters aren't typed, so a check can't tell `[...text]`
  from `[...items]`. One that names what's evidently an array can.
- **Telling apps not to transpile the runtime**: React's Jest config
  transpiles everything, and a bundler's `include` is the app's choice,
  not rust-js's.
- **Array.from for every for-of over a string too**: it makes no
  transform that breaks for-of work, and copies each string first.
