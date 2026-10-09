# 0298. An Option of a value never falsy is tested by its truth

Status: Accepted. Amends [0030](0030-option.md): how an `Option` is tested.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

An `Option` is its value or `undefined` (ADR 0030), and whether it's there
was `o != null`. react.dev tests an object that may be missing by its truth:

```ts
{error && <div>..</div>}
<SandpackConsole visible={!error} />
if (rawError && rawError.message === '..') { .. }
if (message.firstLoad) { .. }
```

## Decision

**An `Option` whose value is never falsy, an object, an array or a
function, is there where it's truthy: `is_some()` is `!!o`, `is_none()` is
`!o`, and a test of one, an `if let Some(..)`'s or a `map`'s, `o`.** A
test reads `!!o` as `o`, and so does JSX where it shows `o && <el />`,
which renders nothing for `undefined`. `!!o ? o.m(x) : undefined` is
`o?.m(x)`, as `o != null ? ..` is.

**A `const` only tests read is read by its truth too**: `const hideContent
= !!error || ..` read only by `if`s, `?:`s and `!`, is `error || ..`, as
react.dev writes it. One read as a value keeps its boolean.

**`o == Some(true)` of an `Option<bool>` is `!!o`**, which a test reads as
`o`: `true`, `false` and `undefined` test alike either way.

- An `Option` of a number or a string, which may be `0` or `""`, is tested
  against `null` still, and so is one of a binding's `unknown` or `any`.
- `is_some_and` and `is_none_or` keep `o != null` and `o == null`: they say
  "there, and" and "missing, or", as react.dev writes `rawError == null ||
  rawError.title === ..`.

## Why

- **It's the JavaScript a person writes** for a value that may be missing.
- **It's the same program**: a value never falsy is falsy only where it's
  `undefined`.

## Consequences

- The react.dev port accepts, by name, the explicit tests of an `Option` of
  a number, `h == null || h === 0` for `!h`: Rust's is exact for `NaN`.

## Amendment: what `filter_map` keeps, and `Boolean`

What `filter_map` and `find_map` keep of a value never falsy is tested by
its truth too, and a callback that only tests its argument's truth,
`(x) => !!x`, is `Boolean`, which is `!!x` and reads no other argument:
`.map(f).filter(Boolean)`, as react.dev's runESLint and TeamMember write
it. A number's or a string's `Option` keeps `(item) => item != null`.
