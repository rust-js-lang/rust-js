# 1072. A nullable field only the crate makes is tested `=== null`

Status: accepted (2026-11-03, rd647 (B), the user's choice (1)). Amends
ADR 0030's `== null` test of a nullable field.

## Context

A `#[rust_js::nullable]` field's `None` is `null` where the crate makes
or sets it (ADRs 0275, 0772, 0810). Its read is still tested `== null`
(ADR 0030), because a JS object may leave the field out. React writes
`clones === null` where the port printed `clones == null`
(measureUpdateViewTransition's `state.clones`). The loose test was printed
everywhere, even on a direct read, `state.x == null`.

## Decision

A nullable field is tested `=== null` / `!== null` where it is `null` or
a value wherever it's read (`mir/nulled.rs`' `null_or_value_field`). That
needs all four of these:

- it's `#[rust_js::nullable]` and not `#[rust_js::optional]`, an `Option`
  or a cell of one (`null_where_made`);
- it's not of a type parameter's value, which generic code stores as it's
  given, `undefined` too;
- one of two things holds:
  - only the crate makes its struct (`pointers().only_the_crate_makes`):
    the crate builds values of it and no code that isn't the crate's
    makes one (`js_may_make`). A struct only JS gives, which may leave the
    field out, is never one;
  - or each constructor of the struct, `Class::of(function::with_this(..))`,
    sets the field first (rd628's `constructed`), so it's there from where
    `new` makes it.

The read says so (`Known::NoneNull`): read whole (`places.rs`), through a
`let` of it, a shared reference, a `RefCell` field's `borrow()` and its
`Deref`, `as_ref()`/`as_deref()`, a `Cell` field's `get()`
(`reads_null_or_value`, `as_ref_of_null_or_value`, consumed by
`mir_nones`), and `is_some()`/`is_none()` through a reference
(`null_or_value_tested`).

So every value the crate makes writes the field `null`. A derived
`Default` and an update's `..Default::default()` now write it `null` too,
as a literal does (`aggregates.rs`' `field_default`, through the
literal's own `nullable_field_made`). Before, they left the key out.

ADR 0262: no observable difference for every value the crate makes, since
each is `null` or a value. The case is a stricter reading of what the
crate already guarantees.

## Why

- It's upstream's test, and it's sound only where no `undefined` can
  reach the field. These are the places that can be proved.
- One fact on the read reuses the existing strict-presence printing
  (ADR 0298), as nullable parameters do (ADR 0861, rd613).

## Alternatives

- **(2) every non-optional nullable field strict, with the `.d.ts`
  required `T | null`**: rd613's promise that JS gives `null`. Rejected:
  a JS caller that leaves the key out, or JSX that omits a prop, would
  read `undefined` as a `Some`.
- **`!js_may_make` alone** (without "the crate builds it"): a struct only
  a JS caller gives, through a `pub fn`'s parameter, isn't
  `js_may_make`, and may leave the field out.

## Consequences

- A struct the crate builds that JS also gives, through a parameter of
  an exported function, is taken to be the program's own data, as ADR
  1022 (rd221) takes it: its nullable fields are tested `=== null`.
- Fields of type-parameter values, optional ones, and those of structs
  only JS gives keep `== null`.

## What holds it

- `test/corpus/nullable_field_tested_strictly.rs` against native Rust:
  - a plain `Option` field, a `Cell`'s and a `RefCell`'s, each made,
    defaulted, updated, `None` and set again;
  - an optional field, which stays loose;
  - a generic cursor whose `push` stores the caller's `None`, `undefined`,
    which a strict test would misread.
- `test/js-constructors.test.ts`, "a nullable field each constructor sets
  first is present": `moduleLoading !== null`, set first, and `inIf !=
  null`, set in an `if`, run with `new Response(undefined, false)`.
- `test/lowering-nullable-reads.test.ts`, "a nullable field is tested ===
  null only where only the crate makes its struct": a struct only JS
  gives, run with `{}`, against a crate-made one.
- Mutations `strict-*` in `lower/mir/nulled.ts`, `lower/mir/dispatch.ts`
  and `lower/mir/places.ts`, and `nullable-field-default-undefined` in
  `lower/aggregates.ts`.
