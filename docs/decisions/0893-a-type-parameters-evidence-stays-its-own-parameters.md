# 0893. A type parameter's evidence stays its own parameters, and the manifest's facts stay typed

Status: Accepted, under the user's delegation of 2026-10-25 ("proceed as
your recommendation"); open to their review. Answers the review's P4, past
its first step ([0886](0886-a-functions-evidence-is-one-ordered-list.md)).

Case ([0262](0262-when-rust-and-js-disagree.md)): none; this is the JS's
shape and the manifest's, not what either does.

## Context

The review's P4 proposed one witness object per type parameter, `function
f(it, t, I, T)` with `T = { Clone, drop, size }`, in place of the
parameters each kind of evidence is now, `function f(it, t, IIterator,
TClone, TSize, dropT)`; and a `facts: BTreeMap<FactKey, Value>` with a
schema version in a library's manifest, in place of a field for each fact.
The problem it named: each new whole-crate fact was a manifest field, an
ABI bump and five modules.

What's changed since: ADR 0886 made the evidence one ordered list,
`evidence_slots`, so a new kind of evidence is one place; and manifest v5
keeps one record per type parameter, `TypeParam { drop, no_drop, facts,
stepped }`, so a new fact of a parameter is a field there, in one place.

## Decision

- **Evidence stays one parameter per piece**, `IIterator`, `TSize`,
  `dropT`. A witness object nests every call site, `f(it, { Iterator: {
  next: $next } }, { Clone: { clone }, drop: dropT })`, which is further
  from the JS a person writes, and a trait's method would read
  `T.Clone.clone(x)` where it reads `TClone.clone(x)`. What a witness buys,
  a library's ABI stable as facts are added, matters when libraries are
  published against a rust-js version; none are yet. ADR 0886's one list
  is where a witness would be made, should that change.
- **The manifest's facts stay typed fields** of `Calling` and `TypeParam`:
  a string-keyed map trades the compiler's checking of each field for a
  version bump less, and v5 already gives each parameter one record.
- **A whole-program link over serialized `Unlinked` trees stays deferred**,
  as the review itself marked it and ADR 0100 names it: it's a release
  build's optimization, not a correctness fix, and `Unlinked` holding no
  rustc types (the architecture test) keeps the door open.

## Consequences

- P4 is done as far as the review's problem goes: one authority for
  evidence (0886), one record per parameter (v5).
- Revisit when a library is published against rust-js, or when a new kind
  of evidence would be the fourth of a parameter's.
