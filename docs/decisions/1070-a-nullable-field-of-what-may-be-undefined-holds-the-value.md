# 1070. A nullable field of what may be `undefined` holds the value itself

Status: accepted (2026-11-03, rd645, the user's choice (B)). Extends ADRs
0051, 0196 and 1017.

## Context

React's unbundled Flight client gives createResponse the manifest's
`serverModuleMap`, Flow's `null | ServerManifest`, where the bundler's
`type ServerManifest = void`. The port declares the type as upstream does,
an untagged enum of one `#[rust_js::undefined]` variant, and the field
`#[rust_js::nullable] server_module_map: Option<&'static ServerManifest>`.

An `Option` of a value that may be `undefined` boxes its `Some` (ADR 0051):
`Some(&ServerManifest::Undefined)` is `$some(undefined)`, since `undefined`
is a `None` everywhere else. A nullable field's `None` is `null`, and its
read tests `!= null`. So the field held a box where upstream holds
`undefined`, and a JS object upstream's caller makes, `{ serverModuleMap:
undefined }`, read as `None`. There was no way to give the field to
createResponse as it is: `js::unknown` takes a value that's always there,
and `js::any_of` refused a type that's known.

## Decision

**A `#[rust_js::nullable]` field of an `Option<V>`, or an `Option<&V>`,
whose `V` is never `null` but may be `undefined` holds `V`'s value
itself.** Its `None` is `null`, its `Some(v)` is `v`, `undefined`
included, and its test is `!== null` / `=== null`. Such a `V` is a `()`
or an untagged enum whose variants are all literals or `undefined`, one of
them `undefined`. The rule is told by the field's declared type alone
(`bindings::holds_value`, `bindings::undefined_never_null`), so every
place that makes, sets or reads it gives the same answer.

It's ADR 1017's held form, which a nested field (`Option<Option<T>>`,
#470) already uses, with `null` for `None`:

- **Made and set**: a struct literal's or an assignment's `None` is
  `null`, its `Some(v)` `v`, and a derived `Default`'s, an update's base
  too, `null`. A literal keeps the key of a `Some`'s `undefined`, `{ map:
  undefined }`, as that `undefined` is no `None` left out (ADR 0280). What
  a local that holds it so holds is stored as it is. A box is unboxed
  where it's stored, `x === undefined ? null : $someValue(x)`, and
  `$some(v)` is `v`.
- **Read**: `is_some()`, `is_none()`, `match` and `if let` test `!==
  null`. A payload read is the value. A read into a local that's only
  tested, read or stored back holds the field's form; any other read is
  boxed where it's read, `x === null ? undefined : $some(x)`, so code
  that takes an `Option` gets ADR 0051's box as before.
- **`js::any_of`** of such an `Option` gives what the field holds, as any
  value is: `js::any_of(options.server_consumer_manifest.server_module_map)`
  prints `options.serverConsumerManifest.serverModuleMap`. Of a box, it's
  what the box holds. `js::any_of` of a value that may be `undefined` or
  `null` as it is, a `()` or such an untagged enum, is allowed too. No
  conversion takes such a value. Every other known type is still
  refused, pointing to `js::unknown_of` (ADR 0909, amended).
- **`.d.ts`**: the field is required, `serverModuleMap: ServerManifest |
  null`.
- **Refused**: `#[rust_js::optional]` on such a field, whose absent key
  would be a `Some`; and a `&mut` to it, which would write a box into it,
  as for a nested field.

ADR 0262's table: the results change if the field holds a box where JS
gives `undefined`, so Rust's behavior is kept, `Some` and `None` told
apart, in the form upstream's JS has.

## Why

- Upstream's object holds `undefined` or `null` there, and its callers
  give it so. The box was a value no JS gives.
- The nested field's held form already had every write and read path,
  and the local analysis that keeps a read unboxed. One form, with its
  `None` a parameter of the type, keeps one rule for "a field held where
  it's made" (`bindings::is_held`).

## Alternatives

- **(A) only: `js::any_of` of a box**, the field still boxed. It prints
  the box's unboxing at each read and keeps `{ serverModuleMap:
  undefined }` a `None`.
- **Every nullable place at once** (parameters, results, thread-locals,
  cells). Fields are what the port needs. The other places keep the box,
  each converting where it meets a field, so no value is misread. Left
  for a later change: see Consequences.
- **A `V` with payload variants**: whether a payload may be `null` needs
  type recognition, which the field-level rule can't ask the same way at
  every place. Such a field keeps the box.

## Consequences

- A nullable `Cell`/`RefCell` field, a nullable parameter, result,
  thread-local or `const` of such an `Option` still holds the box. A
  value passes between them and a field through the box, converted where
  it's stored or read.
- Existing nested-field paths now ask the type which form they hold
  (`held_form`, `held_none`).

## What holds it

- `test/corpus/nullable_field_of_undefined.rs`, against native Rust: an
  untagged enum of one `undefined` variant, one of a literal too, and a
  `()`. Each is made `Some` and `None`, as a constant, a variable, a
  parameter and a derived `Default`. Each is tested, read into variables
  that are tested, given to a function and stored back, then cloned,
  compared and shown.
- `test/lowering-nested-options.test.ts`, "a nullable field of what may
  be undefined holds the value itself": JS-made objects, `{
  serverModuleMap: undefined }` a `Some` and `{}` too, `any_of`'s output
  of a field, a held local and a box, the `.d.ts`, and an enum of `null`
  and `undefined` that stays boxed. "...refuses what would box it": the
  refusals.
- Mutations named for rd645 in `scripts/mutations/lower/mir/nested_options.ts`,
  `mir/dispatch.ts`, `mir/rvalues.ts`, `bindings/nullable.ts`,
  `bindings.ts`, `untagged.ts`, `representation/options.ts`,
  `binding_calls.ts`, `declarations.ts` and `aggregates.ts`.
