# 1161. A present slot is `#[rust_js::present]`; a binding's result of `undefined` alone is `#[rust_js::none_undefined]`

Status: Accepted (2026-11-15), the lead's decision (#1845). Amends
[0280](0280-undefined-fields-left-out.md) and
[0298](0298-truthy-options.md), whose binding-result
`#[rust_js::optional]` is renamed; extends [0810](0810-an-optional-nullable-field-is-absent-until-set.md).
Amended (#1910): a parameter's `None` of `undefined` alone is
`#[rust_js::none_undefined(name)]` on its function.

Case: third ([0262](0262-when-rust-and-js-disagree.md)), results change:
`Object.hasOwn` and `Reflect.ownKeys` tell a key written `undefined` from
one left out, so what a program asks for is what it gets.

## Context

A plain `Option` field's `None` is left out where its struct is made,
`{ value: 3 }`, declared `point?: T` (ADR 0280), and a trailing plain
`Option` parameter may be left out by a caller, `value?: T`. A faithful
port needs the other way too (#1845, Codex's numeric scrub port): a
required key whose value may be `undefined`, `{ value: 3, point:
undefined }`, `point: number | undefined`, which `Object.hasOwn` tells
apart, and a parameter a caller gives, `isISODate(value)`. No form wrote
one. And `#[rust_js::optional]` had two meanings: of a cell's or a nullable
field's key, absent until it's set (ADR 0810), and of a binding's result,
its `None` `undefined` alone (ADR 0298), which a reader of `present`
would take for its opposite.

Measured on the React port (main e285f7b15), "plain is present" as the
default would have added 1206 `k: undefined` keys in 85 files, about 580
of them binding dictionaries (a `PropertyDescriptor`'s `get`/`set`/...,
WebIDL dictionaries), which must stay left out (`{ value, get: undefined
}` throws in `defineProperty`), changed 1639 declared fields and 79
parameters, and needed an exception for component props.

## Decision

- **A plain `Option`, field or parameter, stays omittable**: a field's
  `None` left out, `point?: T`; a trailing parameter's left out by a
  caller, `value?: T | null | undefined`.
- **`#[rust_js::present]` makes a present slot.** Of a plain `Option`
  field: its key written where it's made `None`, `point: undefined`
  (`aggregates`' kept keys), declared `point: T | undefined`
  (`declarations::structures`). Of a function, `#[rust_js::present(value,
  ..)]`, as `nullable(..)` names its parameters: each a parameter a caller
  gives, `value: T | null | undefined`, ending the run of trailing ones a
  caller may leave out (`signatures`). Refused of a cell's, a nullable
  one's or a field that isn't an `Option`, whose key is written already or
  which has none, and of a parameter that isn't an `Option`, each naming why.
- **One axis, two ways**: `present` writes a key that would be left out;
  `#[rust_js::optional]` leaves out one that would be written, a cell's or
  a nullable field's (ADR 0810, the user's cell-shape decision of
  2026-10-27 kept). Each is refused where it's its field kind's default.
- **A binding's result whose `None` is `undefined` alone is
  `#[rust_js::none_undefined]`**, beside `#[rust_js::nullable]`'s `null`
  (`bindings::none_undefined_result`). `#[rust_js::optional]` on any
  function is refused, the message naming it.
- **Declined: the trust of a `pub` function's parameters.** Declaring an
  `Option` parameter `T | undefined` would let JsNones (#1840) trust what
  JS gives a `pub` function, 34 `?? undefined` fewer in the React port
  (`new IntersectionObserver(handleIntersection, options)` as upstream),
  but ADR 0030 declares `None` JS's `null` too, JS callers write `f(null)`,
  narrowing would make the port's declarations stricter than upstream's,
  and a plain-JS caller would still give `null`: #1840's wrong answer back
  behind only a type error. So parameters stay sources, declared `T | null
  | undefined`, and the 34 sites keep their `?? undefined`.

## Every use of `#[rust_js::optional]` at the decision

- Of a field's key, kept: the React port's 108 (react-in-rust 53509261),
  most of them `ReactInternalTypes.rs`' (22) and `ReactTypes.rs`' (17)
  nullable fields, `ReactFlightClient.rs`' debug fields (8), the shared
  internals' (`ReactSharedInternalsClient.rs` 9, `ReactSharedInternalsServer.rs`
  6) and `ReactFiberOffscreenComponent.rs`' (6); and the corpus's
  `optional_of_a_nested_nullable_field_refused` and
  `nullable_field_tested_strictly`.
- Of a binding's result, renamed `#[rust_js::none_undefined]`: the webapi
  generator's 7 (`noneOf`'s `undefined`) and the js crate's `downcast`.

## Consequences

- test/present-slots.test.ts holds a present field's written key and its
  declaration, a plain one's left out, a present parameter's declaration,
  and each refusal; mutations present-key-left-out (`aggregates.ts`),
  present-declared-optional (`declarations/structures.ts`),
  present-param-optional (`signatures.ts`), present-of-any-field,
  present-params-unchecked and optional-of-a-function-unrefused
  (`bindings/validated.ts`).
- Guide: interop/bindings.md's attribute table (`present`,
  `none_undefined`), interop/types.md's `Option` rows, semantics.md.

## Amendment: a parameter whose `None` is `undefined` alone (2026-11-15, #1910)

`#[cfg_attr(rust_js, rust_js::none_undefined(cancelable))]` on a function
says its parameter `cancelable`, an `Option`, has `None` `undefined`
alone, never `null`: declared `cancelable?: C | undefined` where it
trails, `cancelable: C | undefined` before another, which a caller gives.
It's the result's `none_undefined` the other way, the one axis of which
JS value `None` is, beside `#[rust_js::nullable(name)]`'s `T | null`; on
the function, naming the parameter, as `nullable(name)` and
`present(name)` are, as a tool attribute can't sit on a parameter.
Upstream's createDetails declares `cancelable?: Cancelable`, which, under
`strictFunctionTypes`, a plain `Option`'s `C | null | undefined` isn't
assignable from: Codex's type contract asks the constructor be
assignable both ways (test/create-details.test.ts).

- Refused, saying what to write: one naming what isn't an `Option`
  parameter or is a nullable one, and `#[rust_js::optional(name)]` on a
  function, whose message names `none_undefined(name)` (one form per job).
- Not one that isn't trailing: it's a parameter a caller gives as
  `undefined` for `None`, meaningful as it is.
- Its body reads `None` as before, `== null`, so a `null` the declaration
  refuses is `None` too. What's left (#1910): a body that begins `let
  cancelable = cancelable.unwrap_or(<literal>)` printed as JS's default
  parameter, `cancelable = true`, which needs each of the crate's calls to
  give it `undefined` for `None` and the body to read `=== undefined`, as
  a nullable parameter's calls give `null` (ADR 0861).

Case of [0262](0262-when-rust-and-js-disagree.md): none for its JS, which
is what it was; the declaration says what JS may give. What holds it:
test/present-slots.test.ts (the two declarations, omitted, `undefined`,
`true` and `false` run, the three refusals) and test/create-details.test.ts
(upstream's constructor assignable to ours); mutations
undefinable-param-plain (`signatures.rs`), undefinable-param-nullable
(`declarations/functions.rs`), undefined-param-of-any-allowed
(`bindings/validated.rs`).
