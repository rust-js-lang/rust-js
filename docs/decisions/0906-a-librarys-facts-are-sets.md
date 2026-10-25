# 0906. A library's facts are sets of named facts

Status: Accepted, under the user's goal of 2026-10-25 ("implement until
done all the suggested improvements"); open to their review. The review's
P4, its facts map. Supersedes that part of
[0893](0893-a-type-parameters-evidence-stays-its-own-parameters.md), which
had kept typed fields. Library ABI 6.

Case ([0262](0262-when-rust-and-js-disagree.md)): none; this is the
manifest's shape, not what any program does.

## Context

A library's manifest (ADR 0100) told its consumers each yes-or-no fact
of a function as a field of its own: `fails`, `takes_location`, `rest`,
`lazy`, `variable`, and of a type parameter `drop` and `no_drop`. The
review's P4 named the cost: each new whole-crate fact was a field in
the manifest's types, in its writer and in its reader, a check in the JS
tooling, and an ABI bump. It proposed a `facts: BTreeMap<FactKey,
Value>`. ADR 0893 declined that, because a map of strings would lose
rustc's checking of each field.

## Decision

- **Each yes-or-no fact is a variant of an enum, and an item has a set
  of them**: `Calling::facts: BTreeSet<ItemFact>` (`Fails`,
  `TakesLocation`, `Rest`, `Lazy`, `Variable`), and
  `TypeParam::marks: BTreeSet<ParamFact>` (`Drop`, `NoDrop`). Each is
  serialized by its snake-case name, `"facts": ["takes_location"]`, and
  absent when the set is empty. That keeps the review's map, keyed by
  names a schema knows, and keeps rustc's checking, since a key is an
  enum variant and not a string.
- **One reader each**: `Calling::has(fact)` and `TypeParam::has(fact)`.
  A new fact is one variant, set where the contract is written, and
  read by `has`.
- **The JS tooling's mirror (`tooling/manifest.js`) checks each set
  against the names this ABI knows.** `test/manifest.test.ts` holds it
  to the Rust side.
- The type facts a parameter asks (`facts`, the `TypeFact` words) and
  the traits it's stepped by (`stepped`) keep their lists. They are
  already one list each, of names.

## Consequences

- Library ABI 5 manifests are refused, with the message naming 6, as
  any bump is.
- No program's JS changes; std's manifest is regenerated.
- Mutations of the contract's writer and readers now name the sets'
  variants: `library-fails-unlisted`, `library-drops-unlisted`,
  `library-location-unexported`, `library-location-unread`,
  `library-rest-unlisted`, `contract-variable-unsaid`,
  `library-variable-read-as-cell`. Each is caught.
