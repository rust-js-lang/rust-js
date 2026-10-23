# 0676. `js::Kind` tells `undefined` and `null`

Status: Accepted. Amends [0225](0225-unknown-values.md): `js::classify`
takes an `Option<&Unknown>`, and `js::Kind` has `Undefined` and `Null`.
Amends [0668](0668-a-match-of-what-a-value-is-is-a-switch-of-its-typeof.md):
`Undefined` is `case "undefined"`, and `Null` is in `case "object"`.
Amends [0214](0214-untagged-enums.md): an untagged enum's variant without
fields may be `undefined` or `null`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Each variant is the
value itself, as ADR 0214's are, and each test is what JS's `typeof`,
`=== undefined` and `=== null` say of it; a `match` is a `switch` only
where its cases test what its arms' tests did.

## Context

React's `describeValueForErrorMessage` (shared/ReactSerializationErrors.js)
switches on `typeof value`, and `null` is in its `case 'object'`, where
`Object.prototype.toString` names it `"Null"`; `undefined` is its
`default`'s `String(value)`, `"undefined"`:

```js
switch (typeof value) {
  case 'string': ..
  case 'object': {
    if (isArray(value)) return '[...]';
    ..
    return objectName(value);
  }
  case 'function': ..
  default:
    return String(value);
}
```

`js::classify` took a `&Unknown`, never either (ADR 0030), so a port
tested `None` first, `if (value == null) return String(value);`, and
`{a: null}` was `{a: null}` where React's is `{a: Null}`: one `None`
can't be both. React's `getIODescription` writes `case 'object': if (value
=== null) { .. }`. The user decided (2026-10-23): `js::classify` takes an
`Option<&Unknown>`, and `Kind` has `Undefined` and `Null`, so one `match`
tells every JS value as `typeof` and `=== null` do.

## Decision

**`js::classify(value: Option<&Unknown>) -> Kind`; `Kind::Undefined` is
`value === undefined`, and `Kind::Null` `value === null`.** `None` of an
`Option<&Unknown>` stays "`undefined` or `null`" everywhere else.

```rust
match js::classify(value) {
    Kind::String(text) => js::json::stringify(text),
    Kind::Null | Kind::Object(_) | Kind::Array(_) => {
        if js::array::is_array(value) {
            return "[...]".to_string();
        }
        js::object::to_string(value).unwrap()
    }
    Kind::Function(_) => "function".to_string(),
    _ => js::string(value),
}
```
```js
switch (typeof value) {
  case "string":
    return JSON.stringify(value);
  case "object":
    if (Array.isArray(value)) {
      return "[...]";
    }
    return Object.prototype.toString.call(value);
  case "function":
    return "function";
  default:
    return String(value);
}
```

- **One way: `classify(Some(value))` of a value that's there.** A
  `&Unknown` is a type error, rustc's, whose help is `Some(value)`;
  `value.map(js::classify)` is `js::classify(value)`. What's there is
  never `undefined` nor `null`, so where `classify` is given a `Some(..)`
  (`body_queries::given_present`), no test leaves them out:
  `matches!(js::classify(Some(v)), Kind::Object(_))` is `typeof v ===
  "object" && !Array.isArray(v)`, as it was.
- **`Kind::Object` is an object neither `null` nor an array**: of an
  `Option<&Unknown>`, `typeof value === "object" && value !== null &&
  !Array.isArray(value)`, `null` first, as JS writes it.
- **A variant without fields of an untagged enum may be `undefined`,
  `#[rust_js::undefined]`, or `null`, `#[rust_js::null]`**, as `Kind`'s
  are; one without either is its name's string (ADR 0214). Each is the
  value, `===` tells it, never `== null`, which is both. An `Option` of
  such an enum is a box (ADR 0051), as `Some(Missing)` isn't `None`.
  `untagged::unit_value` is the one answer to what such a variant is.
- **In a `switch (typeof value)`** (ADR 0668): `Undefined` is `case
  "undefined"`. `Null`, which `typeof` says is an object:
  - in an arm that takes the objects, `Null | Object | Array`, is its
    `case "object"`;
  - alone, is `if (value === null) { .. }` first in the case of the arm
    that takes the objects, as React writes it; where none does, an `if`
    before the `switch`, as an array's is;
  - `Undefined | Null` is `if (value == null)` before the `switch`, as an
    `Option`'s `None` is tested (ADR 0030);
  - with another case, `Null | String`, or where `null` is `_`'s and an
    arm is `Kind::Object`'s, it's an `if` chain: `case "object"` would
    take `null`.

## Why

- **It's what React writes**: `null` an object in its `typeof` switch,
  `"Null"` of `Object.prototype.toString`, and `undefined` the `default`.
- **One `match` of every value**: TypeScript's `unknown` is `undefined`
  and `null` too, and `typeof` and `=== null` tell all eight.
- **`classify(Some(value))`, not a second function** of what's there: one
  way to ask, and rustc says it.
- **Attributes, not names**: `Null` and `Undefined` are what the variants
  are named, as a program's own enum may name them otherwise, `Missing`.

## Alternatives

- **`classify(&Unknown)` and `classify_option`**: two ways to ask one
  question.
- **`#[rust_js::name = null]`**: an attribute's value is a literal, and
  `null` isn't one.
- **`null` always an `if` before the `switch`**: reads as no JS does where
  an arm takes the other objects; React tests it in `case 'object'`.

## Consequences

- **A `match` of `js::Kind` without `_` is an error until it names
  `Undefined` and `Null`**, rustc's exhaustiveness, as intended.
- A call of `classify(&Unknown)` is an error: `classify(Some(value))`.
- `same_test` no longer takes `x === null` for `x === undefined`, which
  only `==` makes alike; a constant's `==` of such an enum is `===`.

## What holds it

- `test/lowering-switches.test.ts`, "a match of what a value is tells
  undefined and null", run in Node of each kind, and "classify of a value
  that's there is of its Some"; ADR 0668's, of `classify(Some(value))`,
  as it was, and `Undefined | Null`'s `== null`.
- `test/corpus/untagged_null_undefined.rs`: a program's own enum of
  `undefined` and `null`, native and JS.
- Mutations in `scripts/mutations/lower/untagged.ts`,
  `scripts/mutations/lower/mir/tidy/switches.ts` and
  `scripts/mutations/lower/body_queries.ts`.
