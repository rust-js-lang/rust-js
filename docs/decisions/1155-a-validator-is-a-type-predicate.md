# 1155. A validator is a TypeScript type predicate, `js::Is<T>`, proved of its parameter

Status: Accepted (2026-11-15, #1828, the form the lead approved: one
constructor, `holds()` its one reader, `unknown` parameters alone).

Case ([0262](0262-when-rust-and-js-disagree.md)): none. The JS is the
boolean the validator was; only its declaration says more.

## Context

Upstream's calendar (vercel-labs/ui@220c1a0,
`packages/core/src/foundation/date/plain-date.ts:10,26`) writes:

```ts
type ISODate = string
function isISODate(value: unknown): value is ISODate
```

The port's validator returned a `bool`, declared `boolean`, so Codex's
consumer (rust-js-lang/vercel-ui-sdk-in-rust@da281b6,
`tests/types/calendar.ts`), `if (isISODate(value)) { const date: string =
value; }`, didn't narrow. A predicate is a claim TypeScript trusts: free
declaration text, an attribute naming the parameter, would be a claim no
body proves.

## Decision

**A function that tells whether a value is a `T` returns a `js::Is<T>`,
which only `js::Is::when(value, test)` makes**, and it's declared `value is
T` of the parameter its body gives `when`:

```rust
#[cfg_attr(rust_js, rust_js::name = "isISODate")]
pub fn is_iso_date(value: Option<&js::Unknown>) -> js::Is<IsoDate> {
    js::Is::when(value, |text| year(text).is_some())
}
```

```js
export function isISODate(value) {
  return typeof value === "string" && year(value) != null;
}
```

```ts
export function isISODate(value?: unknown): value is ISODate;
```

- **It can't lie of what it's given.** `js::Is` is `#[repr(transparent)]`
  over a private `bool`, JS's boolean (ADR 0584), made only by `when`,
  whose binding form tests that the value is a `T` first, as an untagged
  enum's variant is told (`kind_test_of`, ADR 0214's kinds), `typeof value
  === "string"` of text, `value instanceof Date` of a class's, then runs
  `test` of what a `T` lends, `T: Borrow<V>`, as `HashMap::get` takes a
  key, a `String`'s `&str`. A `T` JS can't tell is a compile error.
- **Of a parameter, proved of MIR** (`predicates::guarded`, the one owner
  lowering and declarations ask): `when` is given one of its function's
  own parameters as it was passed, never changed (`mut`), the same one of
  each `when`; a function that gives a `js::Is` makes it so, not another's.
  Each refused in every build, not only a declarations build.
- **Only an `unknown` parameter.** TypeScript narrows a predicate both
  ways: `if (!isISODate(v))` takes `ISODate` out of `v`'s type, so of a
  `string | number`, the false branch would say a string that isn't a
  date is a number. Of an `unknown`, `Option<&js::Unknown>`, the false
  branch narrows nothing, upstream's case. Of any other value the refusal
  says to return a `bool`, declared `boolean`, and that a predicate that
  narrows both ways, a pure checked downcast of any parameter, is #1828's
  to reopen: no second constructor until a port needs one.
- **`holds()` is its one reader** in Rust, the value itself.
- **Its type as written**: `value is ISODate`, the alias by its
  `#[rust_js::name]` (#1829, ADR 0196 amended), not `string`.

## Why

- **The declaration says what the body does**: true only of a `T`, and of
  the parameter that's tested; a predicate TypeScript trusts can't be
  wrong of either.
- **The JS is upstream's**, the `&&` of the kind test and the validator's
  own, inlined, no helper.

## Alternatives

- **`#[rust_js::guards = "value"]` on a `bool` function:** free text no
  body proves.
- **Returning `Option<&T>`:** idiomatic Rust, but its JS is the value, not
  upstream's boolean API.
- **`bool::from(is)` beside `holds()`:** two readers of one value.
- **A second constructor for a checked downcast alone, `js::Is::of`:**
  sound both ways of any parameter, but no port needs it yet.

## Consequences

- `test/js-predicates.test.ts`: the JS and declaration above; Rust's
  answers of a date, a non-date string, a number, `undefined` and `null`,
  and `holds()` counting them; Codex's calendar.ts under strict tsc; the
  false branch, `const text: string = value` of `!isISODate(value)`,
  refused by tsc (TS2322); and refused, each saying what to write: a
  `&str` made `unknown`, a `mut` parameter changed first, two parameters,
  and another function's `js::Is` given on.
- Mutations guard-of-a-changed-parameter, guards-of-two-parameters,
  predicate-made-elsewhere, is-when-untested, predicate-declared-any and
  predicate-of-the-type-resolved.
- The declaration model and its printer have `predicate`.
