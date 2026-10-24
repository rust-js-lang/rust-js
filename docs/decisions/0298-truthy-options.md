# 0298. An Option of a value never falsy is tested by its truth

Status: Accepted. Amends [0030](0030-option.md): how an `Option` is tested.
Amended: an `Option<bool>`'s `is_some_and` of its truth is `!!o`, and a
test reads `x ?? false` as `x`, and what `!` turns or a `while` tests as
an `if`'s test.

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
`o?.m(x)`, as `o != null ? ..` is. (Amended: the `Option`'s `map` says so, which knows
it's never falsy; JS's `!!s` alone may be text's, `!s.is_empty()`, which
`""` fails, and `s ? s.m() : undefined` is no `s?.m()`.)

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
it. A number's or a string's `Option` keeps `(item) => item != null`. And `?` of one is
`if (!o) return;`, as react.dev's runESLint writes `if (!error)`.

## Amendment: an `Option<bool>` that's `Some(true)`, tested

React's `ReactNoopUpdateQueue` asks whether it warned of a key,
`js::dict::get(did_warn, &key).is_some_and(|&warned| warned)`, which was
`const option = didWarn[key]; option != null && option`, read twice, where
upstream writes `if (didWarnStateUpdateForUnmountedComponent[warningKey])`.
And `flag.unwrap_or(false)` in a test was `if (flag ?? false)`.

- **`is_some_and` of the truth of what it holds** (`tests_its_truth`, an
  `Option<bool>`'s `|&w| w` or `|v| js::truthy(Some(*v))`), its `Some`
  not boxed, nothing to drop, is `!!o`: `o != null && o` read once, as
  what's not there is falsy too. A test reads it as `o`, as it does `o ==
  Some(true)`'s `!!o`.
- **A test reads `x ?? f` as `x`, `f` a falsy constant** (`Expr::tested`,
  the one answer to what a test reads): `undefined` and `null` are falsy,
  as `false`, `0` and `""` are. `if (flag)` of `flag.unwrap_or(false)`.
- **What `!` turns is read as a test**, wherever it is, `const off =
  !flag`, and so is a `while`'s test, `while (flags[i])`, as an `if`'s
  is.

Case: A, as before: each reads true where the other does, for any value.

Held by `test/corpus/option_bool_tested.rs` against native Rust, and
the mutations `is-some-and-truth-twice` (`scripts/mutations/lower/combinators.ts`),
`tested-coalesce-kept` (`js/expr_walk.ts`), `falsy-constant-false-truthy`
(`js/make.ts`), `not-operand-untested` and `while-test-untested`
(`prepare.ts`).
