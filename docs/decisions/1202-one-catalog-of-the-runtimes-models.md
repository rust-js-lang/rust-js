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

## Amended: step 2a, the catalog's rows of `$stdSites` (2026-11-25)

The runtime's helpers that read `$stdSites` were mostly outside the catalog:
of 51, 31 had no entry and only 6 were bound, because step 1 seeded it from
the twin net, which sees the iterator tables' helpers alone. And which std
function each row's sites are of was a second table, `RAISED` in
`compiled_std/raised.rs`, beside the catalog.

- A model stands for std items of two kinds, `Stands::Path`, a std item at
  its path, and `Stands::Row`, the function whose panics a row of
  `$stdSites` holds, by the row's name. Each helper that reads a row names
  it, so the rows keep their names (no output moves) and each is its
  readers' key.
- The lowering's own readers of a row, which write a form in place of a std
  body (`clonedAt`, `vecDequeRange`, `ordClamp` and seven more), are
  entries too, `Kind::Form`. A row a helper and the lowering both read is
  the helper's entry, its note naming the lowering's file.
- `RAISED` is the catalog's `models/rows.rs`, `ROWS`, split from models.rs
  by concept; `bun run std` walks it as before, and refuses a catalog whose
  model or form reads a row nothing writes (`rows_unwritten`).
- A new reason, `outside`: a std function no table of the twin net reaches
  yet, a `Duration`'s, UTF-8's, an I/O type's, or an iterator's no table has
  a case of. The twin net doesn't take such a model off where no case
  imports it, as no case is meant to yet; a form is no helper of the
  runtime's, and the twin net doesn't judge it.
- The runtime exports 1,529 helpers and the catalog names 346: the other
  1,183 are ratcheted (`test/twin-net.test.ts`), so a helper added to the
  runtime is catalogued with it, and each later step catalogs what it
  touches. The unbound models' ratchet falls to 207, the 13 site readers
  that were unbound now bound to their rows.

Step 2b makes each model's rows its function's, walked from the catalog;
step 2c checks each bound model reads every site of its function, or the
catalog says why one is unreachable (#2509, #2488's class).

## Amended: step 2c-1, each row read through its handle (2026-11-25)

Step 2a found the lowering's readers of `$stdSites` by the text
`std_site("…")`, and missed 14 that name their row by a string passed as a
value (`Some("vecDrain")`, `("rcFromTrustedLen", "rcSliceLayout")`), and
`$fmtOrPanic`, which reads `$stdSites[kind]` by the name `format_args.rs`
gives it. And `fnMutCall`, written by `bun run std`, had no reader at all
since d00ee33dc, whose `clonedAt` took its place.

- Each row is a variant of `StdRow` (`recognition/models/rows.rs`), its
  name in `$stdSites` its one spelling; `ROWS`, `HELD_STEPS` and `clonedAt`
  are written by it, and the catalog's `Stands::Row` names it.
- The lowering reads a row through its handle alone, `StdRow::site`,
  `site_of`, `written` and `text` (`std_rows.rs`, a lowering module, as
  recognition reaches no runtime): no `Expr::std_site` or
  `runtime::std_site_*` call elsewhere (test/architecture-models.test.ts).
  A row's name still reaches JS as text where the runtime reads it so,
  `$fmtOrPanic`'s `kind`, given as `StdRow::Format.name()`.
- `bun run std` refuses a table whose rows aren't `StdRow`'s, both ways
  (`rows_checked`), and a row no model or form of the catalog names
  (`rows_unnamed`); a unit test holds each to it both ways.
- The 14 are `Form` entries, `$fmtOrPanic` a model of `format` and
  `toString`, and `fnMutCall` is gone from `ROWS` and the table; no JS a
  program reads moves.
- `clonedAt`'s keys, one of each std `Clone` impl, are held to std's MIR by
  test/std-cloned-at.test.ts, not a corpus case each; a few corpus cases
  cover its reader, as its entry says (the lead's choice, (b)).

Step 2c-2 triages the 20 sites of other rows no runtime text or committed
JS reads, against native: a wrong line is a wrong answer, filed and fixed
as a P0; an unreachable one is listed with its reason.

## Amendment: step 3a, the slice searches' reason (2026-11-25)

A slice's `chunk_by`, `chunk_by_mut`, `split` and `split_mut` iterators
(`$lazyChunkBy`, `$lazyChunkByMut`, `$lazySliceSplitBy`,
`$lazySliceSplitByMut`) were `Unproven(Iterators)`, as if std's build only
lacked their rows. Given them, it refuses their bodies: their `next`
steps the slice by its pointers, `NonNull<[T]>`, which reach memory. So
their reason is `Memory`, each note saying so, and `Split` and `SplitMut`
are bound. Proving them through std's build needs it to model a slice's
pointers (#1458). `char::decode_utf16`'s `DecodeUtf16` compiles there,
but generic code's `IntoIterator` handed to it doesn't cross (#2598), which
Campaign 6's dictionaries crossing answers; it's left unproven until then.


## Amended: each row says the net that holds it (2026-11-25)

The twin net's whole run took off models.rs every unpitted row no case
imports (`unshare`), as if the twin net were every row's one holder. It
isn't: #2403's 51 boundary checkers (`$checks`, `$checkU8`, `$brand`, …)
are glue no twin case reaches, as a case's functions take no value from JS,
and the boundary's tests hold them; so the nightly's bless would have
proposed taking all 51 off, and the runtime's exports' ratchet would then
fail. Step 2a had met the same question for the site readers and answered
it by a reason, `shared[helper] !== "outside"`: a second rule of who holds a
row, read from why it's unproven.

- Each row says which net holds it, `Entry::held`, a `Net`: `Twin`, the
  twin net (ADR 1122); `Boundary`, the boundary's tests (ADR 1192); or
  `StdSites`, `bun run std`'s sites, which refuse a row the catalog names
  no reader of (`rows_unnamed`, step 2c-1). `entry(helper, kind, note)`
  stays the one constructor, held by `Twin`; another net's row reads
  `entry(..).held(Net::Boundary)`. `rust-js --std-models` prints it,
  `"heldBy"`, and the twin net reads it there alone.
- The drift takes off only a row the twin net holds. The `outside` rule is
  gone: `outside` stays a reason, not a drift rule, and each site reader
  `outside` the tables is `.held(Net::StdSites)`. Forms are no helper of
  the runtime's and were never shared.
- A twin case that pits a row another net holds pits it still: its bless
  writes it `Pitted`, held by the twin net from then (its `.held(..)` gone),
  as the twin net then answers for it.
- Tests (test/twin-net.test.ts): "a whole run that imports no row another
  net holds keeps it" (a site reader, `$durationAdd`, and a boundary check,
  `$checks`) and "a row another net holds that a case pits is blessed
  `Pitted`, the twin net's". Mutations: `twin-outside-unshared`, moved to
  the holder's filter; `models-held-all-twin`, every row printed `twin`;
  `twin-bless-blind-to-held-rows`, the bless not reading a held row's line.
