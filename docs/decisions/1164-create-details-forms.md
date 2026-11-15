# 1164. The forms upstream's createDetails needs

Status: Accepted (2026-11-15, the lead's decisions on Codex's contract,
rust-js-lang/vercel-ui-sdk-in-rust@2680f613e,
docs/create-details-contract.md). Each form is a section, landed as it's
built.

Case ([0262](0262-when-rust-and-js-disagree.md)): none for each form's
JS, which is what it was; the declarations change, to say what it is.

## Context

The vercel UI kernel's `kernel/details.ts` (vercel-labs/ui@220c1a0) is
the details every `on<Key>Change` callback receives: a vocabulary of
reasons, `ReasonVocabulary`, each with the DOM event behind it; the
details of a change, `ChangeDetails<Reason>`, one member per reason so
checking `reason` narrows `event`; and `createDetails<const Given,
Cancelable extends boolean = true>(given, source, cancelable)`, whose
result keeps `given`'s own keys, then `source`, then a detachable
`cancel` and a live `get canceled()`. Codex's contract asks for its
declarations and ABI as they are. The lead approved seven forms (A to G)
and rejected a default-parameter print (F as first proposed) and a
flatten of a reference (G as first proposed); this records each.

## A. A trait's impls as one type map

**A unit struct named as a string is that string, its type as its value
is**: `#[cfg_attr(rust_js, rust_js::name = "trigger-press")] pub struct
TriggerPress;` is `"trigger-press"` in JS (ADR 0013), and now `export
type TriggerPress = "trigger-press";`, so a function giving one is typed
`TriggerPress`, no longer `any`. Its `rust_js::name` is its value, not
its type's name, but for a class's or a namespace's (`named_unit`,
`type_name`).

**`#[cfg_attr(rust_js, rust_js::type_map(map = "ReasonVocabulary", keys =
"ChangeReason", lookup = "EventOf"))]` on a trait of one associated type**
declares its impls for such structs as one object type, its keys and a
lookup of it (`declarations/type_maps.rs`):

```ts
export type ReasonVocabulary = {
    /** Click, tap, Enter or Space on the part that opens. */
    "trigger-press": PressEvent;
    /** Escape. */
    "escape-key": KeyboardEvent;
};
export type ChangeReason = keyof ReasonVocabulary;
export type EventOf<Reason extends string> = Reason extends ChangeReason ? ReasonVocabulary[Reason] : Event;
```

- Each key is an impl's struct's name, its value the impl's associated
  type, in the order the source writes the impls, with the struct's doc
  comment, which a caller's editor shows.
- The lookup's other strings take the value of the trait's impl for
  `&'static str`, an open reason, upstream's `: Event`; without one,
  `never`. Its type parameter is the trait's name, as upstream's is.
- The impls are the one source of all three: no second list of reasons.
- Refused, saying what to write: a `type_map` without `map`, a `lookup`
  without `keys`, a trait of other than one associated type, and an impl
  of a type that's neither a named unit struct nor `&'static str`.
- The printer gains a conditional type, and a type alias of an object
  prints its members' doc comments, as an interface's do.

## C. A struct declared of each of a union's members

**`#[cfg_attr(rust_js, rust_js::distributive)]` on a generic struct of one
type parameter** declares it a distributive conditional type,
`CommitDetails<R extends string = string> = R extends unknown ? { reason:
R; event?: EventOf<R> | undefined; .. } : never`, so a TypeScript caller's
check of `reason` narrows `event`, as upstream's `ChangeDetails` and
`CommitDetails` are one member per reason. Of another number of type
parameters, or flattening a struct its declaration would extend, it's
refused.

- **A type parameter's default is TypeScript's**: `R: Reason = &'static
  str` is `R extends string = string`, so `CommitDetails` without its
  reason takes any. Every struct's default is declared now, where it was
  left out; the printer passes TypeScript's native factory its default in
  the right place.
- **A type map's associated type is its lookup**: `<R as Reason>::Event`
  is `EventOf<R>`, or without a lookup `ReasonVocabulary[R]`, where it was
  `any`, so the field's event is the reason's.
- A distributive alias's members carry their doc comments, as an
  interface's do.

## Consequences

- test/declarations-type-maps.test.ts: the declarations, a strict
  TypeScript caller (literal reasons, the lookup of a known and an
  unknown reason, the negatives), and the refusals;
  test/typescript.test.ts's conditional type and object alias's docs.
- Mutations trait-type-map-undeclared, named-unit-type-its-value,
  named-unit-not-declared (`declarations.rs`),
  type-map-lookup-of-others-never, type-map-keys-unwritten,
  type-map-members-undocumented (`declarations/type_maps.rs`),
  object-alias-members-undocumented, conditional-type-unprinted
  (`typescript/index.js`); transparent-tuple-struct-any and
  type-name-given-unread retargeted.
- C: declarations-type-maps' `CommitDetails` (its declaration, a strict
  caller whose `reason` narrows `event`, the default, a wrong event
  refused); typescript.test's default and distributive alias; mutations
  distributive-declared-as-interface (`structures.rs`),
  struct-type-default-undeclared, type-map-projection-any
  (`declarations.rs`), type-parameter-default-unprinted,
  distributive-members-undocumented (`typescript/index.js`);
  object-alias-members-undocumented retargeted.
- Guide: interop/export-to-javascript.md, TypeScript declarations.
