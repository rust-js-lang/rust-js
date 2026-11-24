# 1202. One catalog of the runtime's models: which std item each stands for, and what proves it

Status: Accepted (2026-11-25, Campaign 5's step 1, #1458 and #2513; the
lead's call, on Tu's behalf).

Case ([0262](0262-when-rust-and-js-disagree.md)): none. No program's JS
changes. This decides where the facts that later steps gate output on are
kept.

## Context

The runtime re-implements std by hand: adapters, ranges, collections'
methods, a `RefCell`'s borrows. #1458 asks that each hand model be used
only where something proves it equal to std's own compiled body. #2513
asks that each model's panic sites, messages and capacity checks come from
its std function's MIR, not be chosen by hand. Both need the same fact
first: which std item each model stands for. Nothing recorded it. The twin
net (ADR 1122) kept three lists of its own in test/twin-net.ts: `pitted`,
the 46 helpers a case runs against std's compiled body; `shared`, 257
helpers both builds import, each with a reason; and `why`, the reasons'
texts. It named no std item.

Measured on main a93d466b4 for the campaign's plan: compiled std as the
iterator default is correct (every corpus case compiles, and
iterator-forms' `differs` is empty) but costs 14% more lines, a p90 of
×2 among changed cases, and up to ×13. So the flip is per model, gated
by its proof, and each model needs its std item and proof in one place.

## Decision

- **One catalog,** `crates/rust-js-lower/src/recognition/models.rs`,
  `MODELS`: each runtime helper, on one line, is `Glue` (rust-js's own:
  how JS holds a value, a reference, a panic, a drop or a dictionary, no
  copy of a std body) or a `Model` of the std items it stands for, with
  its proof. The proof is `Pitted` (the twin net runs a case of it
  against std's body) or `Unproven(Why)`, the twin net's reasons, now an
  enum whose texts live there.
- **Std items by their public paths,** `core::iter::Rev`, or a trait's or
  a type's method, `core::iter::DoubleEndedIterator::rfold`. Each is
  resolved where `bun run std` reads std (`models::unresolved`, asked by
  `raised_catalogue`): one that names nothing fails the std build, so a
  toolchain upgrade that moves one is said at once.
- **Seeded once** from the twin net's lists, which are then deleted. 45
  models are bound now: the adapters, ranges, `Repeat`, `StepBy`, `Fuse`,
  the double-ended and exact-size methods, `Chars`, a slice's `Iter`,
  `Peekable::peek`, a `RefCell`'s borrows and the slice searches' structs.
  The other 220 stand for no item yet, which is #2513's measure of what's
  left to bind (test/twin-net.test.ts prints it and holds it to at most
  220). Each later step binds what it touches. Step 3 can't flip a model
  that has no binding.
- **The compiler prints it,** `rust-js --std-models` (JSON), as
  `--runtime-module` and `--std-declarations` print theirs. The twin net
  reads it there: no TypeScript mirror, so nothing to keep in step.
- **The twin net blesses drift into models.rs.** A helper a whole run now
  pits becomes `Pitted`, keeping its std items; a new one gets its line in
  name order; one nothing imports is taken off (`blessedCatalog`).
- **No second list** (test/architecture-models.test.ts): models.rs alone
  of the compiler's sources declares the catalog, and no TypeScript file
  under test/ or scripts/ holds a helper's proof or reason, or a
  `pitted`/`shared`/`why` table.
- **By bare name:** the catalog names each helper as `rev` for `$rev`,
  the `$` the runtime exports it by put on where it's printed. So the
  architecture rules that find where a helper's JS form is written find
  no name the catalog only knows, and none names the catalog as an
  exception.

## Consequences

- Campaign 5's step 2 generates each bound model's panic sites from its
  std item's MIR, and step 3 flips an unproven model to compiled std,
  adapter by adapter, each with its snapshot moves brought to the lead.
- A helper the runtime gains must be added to the catalog, as glue or a
  model, or the twin net names it as a gap.
- The nightly's twin-net bless patch now edits models.rs, so the compiler
  is rebuilt to print what it wrote.
- Held by test/twin-net.test.ts (the catalog's helpers are the runtime's,
  the measure, the verdicts and the bless), test/architecture-models.test.ts,
  `bun run std`'s path check, and the mutations `models-trait-method-unresolved`,
  `models-glue-as-a-model`, `models-pitted-unsaid`, `twin-bless-keeps-unshared`
  and `twin-bless-pit-unwritten`.
