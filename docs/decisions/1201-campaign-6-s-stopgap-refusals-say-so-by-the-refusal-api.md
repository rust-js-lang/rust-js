# 1201. Campaign 6's stopgap refusals say so by the refusal API, each cell mapped once to its open issue

Status: Accepted (2026-11-25, #2539 and #2557, the lead's call).

Case ([0262](0262-when-rust-and-js-disagree.md)): none. No program's JS
changes; this holds where the compiler refuses. One refusal's words
change: the `extend` refusal of #849 said itself past `refuse`, and now
says what every refusal says.

## Context

Campaign 6 (#2539) gives each form and fact a conversion at each place a
value crosses into or out of generic code. While it's built, a wrong
answer found at a crossing is closed by refusing the program. That's
correct: a refusal beats a wrong answer. In one day, four refusals of
idiomatic Rust whose behavior JS can keep came this way: #2532's
`impl_self_mut_object`, #2534's `refuse_placed_item`, and #2542's zip in a
library and in a closure. Their bugs closed with the refusals, so nothing
tracked lifting them.

Listing them took a grep of refusal messages read by hand. The refusal API
(`refusal::refuse`, a lowerer's `unsupported`) said nothing of whether a
refusal is of what Rust's behavior can't be kept for or a stopgap. A list
kept beside the code, by fragments of the messages, would have been a
second catalog that a new, unlisted stopgap slips past (#2557).

## Decision

A stopgap says so where it's made, by the refusal API:

- `refusal::Crossing` names each cell of the matrix: a form or a fact, and
  the crossing it isn't converted at yet.
- `Refused::Unconverted(crossing, &said)` refuses as `said` does, in the
  same words, help and note. `unconverted(span, crossing, what)` is
  `unsupported`'s form of it.
- `Crossing::lifted_by` maps each cell to the one open issue whose fix
  lifts it, the one place each is mapped, as `refusal::issue_of` maps open
  calls. On 2026-11-25 there are 16 cells and 33 refusals: eight cells got
  issues of their own that day, #2549 to #2556; the others had open ones
  already.

Three checks hold it:

- test/architecture-stopgaps.test.ts holds each cell mapped once, to an
  issue no other cell has, so a fix closes a cell. Each cell must be
  named by a refusal, and their count is exact. A cell whose refusals are
  all lifted is removed, so the cells only shrink. A refusal added or
  lifted changes the count where it's reviewed.
- `bun scripts/stopgap-refusals.ts` reads `lifted_by`'s arms and checks
  each issue is open. The issue-labels workflow runs it, with `--mark`, as
  an issue closes and daily. An issue closed while its cell's refusals
  remain on `main` fails the run and is told so on the issue, once.
  Tests can't ask GitHub, which is why the workflow runs it.
- A new stopgap is made as `Unconverted` with its cell, a new cell with
  its open issue.

Campaign 6 ends when `Crossing` has no variants left.

Refusals of what Rust's behavior can't be kept for aren't stopgaps, and
neither are gaps that belong to other owners (std's panic locations,
props, React, serde, C-variadics).

## Consequences

The `extend` refusal of #849 and #1180 was emitted by `span_err` with words
of its own, "rust-js: this call gives generic code …". It now goes through
`refuse`: "rust-js does not support a call giving generic code … yet",
with its advice as help. Three diagnostics rows assert the new words.

A refusal that's a stopgap but is made without `Unconverted` still isn't
found. The test can't tell it from a refusal of what can't be kept: that
is the author's call, made where the refusal is written and reviewed.

A length or discriminant "only a caller knows" (a generic array's length,
a chunk's, `mem::discriminant` of a type parameter) isn't a cell yet. Those
refusals become cells if `ParamForm` takes const facts.

## Amended 2026-11-25: write results as an iterator's items (#2560)

A wrong answer of Campaign 6's matrix was closed by a stopgap, a cell of
its own:

- `Crossing::TaggedItem` (#2566): write results as the items of an
  iterator given to generic code that reads them as `Result`s of its type
  parameters, `I: Iterator<Item = Result<T, E>>`, an `IntoIterator`'s, or
  inside them, `Option<Result<T, E>>`. Items cross as their caller holds
  them, `undefined` or the error, so `r.is_ok()` read `.TAG` of
  `undefined` (#2560). `generic_facts/refusals.rs`' `refuse_tagged_item`
  refuses the call. std's code given them, a collect into a `Result`,
  reads them tagged where it's called (`std_reads_tagged`), and never
  comes this way. The call's one check of what crosses unconverted,
  `refuse_placed_within`, is renamed `refuse_unconverted_call`, as it
  asks both. Lifting the cell tags each item where it crosses.

It follows ADR 0262's last row: the results changed, and until the
crossing converts them, a compile error. There are now 17 cells and 35
refusals.

## Amended 2026-11-25: a write's result held within, a cell of its own

`held_write_result`'s two refusals were said by `unsupported`, outside the
cells: a write's result held inside what generic code has as a `Result`
of its type parameters, a `Vec<Result<T, E>>` given (`assign/boxed_args.rs`)
or given back (`mut_refs.rs`), `{ TAG }` there where its caller holds
`undefined` or the error (#1416's first shape). Each is a crossing that
can't convert a form yet, so it's a stopgap, and it's said through the
cell `Crossing::TaggedWithin`. The cell is lifted by #2568, split from
#1416 because no two cells share an issue (#1416 stays
`TaggedArgument`'s), and #2568 converts at that boundary. Their words
name #2568 now. There are now 18 cells and 37 refusals.

## Amendment: the facts only a caller knows are a cell (2026-11-25, #2570)

Tu decided on 2026-11-25 that the facts of a type or a const only a caller
knows are Campaign 6's, as a step 7 after step 6, measured by the
refusals it lifts. They're one cell, `Crossing::TypeFact`, lifted by
#2570. It has nine refusals: an array of a generic length, a chunk's
length, `mem::discriminant`, `type_name`, `size_of`/`align_of` of a type
parameter, a map's class, an unstable and a stable sort's layout, and an
own provided method given a const (#2421).

A type parameter's `TypeId` isn't in it: it's a `TypeFact` asked through
the dictionary, `Crossing::FactDictionary`'s. "The `TypeId` of this `dyn`"
isn't either, because it's of a `dyn` without `Any`, which no caller can
give. The words of each refusal are as they were.

## Amended 2026-11-25: a write's result an impl's dictionary gives (#2559)

Another wrong answer of Campaign 6's matrix was closed by a stopgap, a
cell of its own. `Crossing::ValuedDictionary` (#2565): an impl whose
method gives a write's result, `fn make(&self) -> fmt::Result`, where
the trait's signature, as the bound the dictionary is given for names
it, gives a type parameter or an associated type, `Self::Out`. The
method throws its error (ADR 0187), and generic code calling it through
the dictionary holds `M::Out` as a value (`Form::Valued`, #1410), so
`made_twice(&Bad)` threw `$FmtError` (#2559). `generic_facts.rs`'
`impl_entry_valued` says which method: the impl's own signature as it's
compiled, `-> Self::Out` normalized, against the trait's instantiated
with the bound's arguments. `generic_facts/refusals.rs`'
`refuse_valued_entry` asks it twice. `impl_dictionary` asks it of the
bound as its caller names it, `Bad: Make`, wherever the dictionary is
made, a `dyn`'s and a supertrait's too. Each call's evidence asks it of
the bound as generic code declares it, `W: Is<T>`, whose `T` it holds as
a value. Where the bound names the write's result, `impl
Is<fmt::Result>`, generic code calls the method as a write and catches
its error (#1634), so that compiles. Two shapes that ran right are
refused now, as nothing at the crossing tells them apart: a `dyn
Make<Out = fmt::Result>` only concrete code calls, and generic code that
gives `M::Out` straight back, `fn made<M: Make>(m: &M) -> M::Out`, whose
caller catches the error the method throws
(`write_result_through_a_dictionary` lost its `made`). Lifting the cell
gives the entry's result as a value, as `entry_writes_tagged` gives
`Form::Tagged`'s.

It follows ADR 0262's last row too. There are now 20 cells and 47
refusals.

## Amended 2026-11-25: five refusals said outside the cells

A scan of the refusals said by `unsupported` found five that are
stopgaps, each a crossing that doesn't convert a form or a fact yet. Each
is said through a cell now, its words unchanged:

- `traits/calls.rs`' `evidence_args_given`: a `RefCell` of a `dyn` of the
  crate's trait, its `{ value, impl }` pair (ADR 0397), given where
  generic code holds a cell of a type parameter, which reads a cell of a
  value (ADR 0451). A new cell, `Crossing::DynCellArgument`, lifted by
  #2569.
- `traits/calls.rs`' `provided_on_steps`: a fact an override of an
  iterator's provided method asks that generic code's caller's dictionary
  doesn't carry (#2442), `Crossing::FactDictionary` (#2555).
- `traits/calls.rs`' `refuse_fact_untaken`: a fact a consumer's impl of a
  library's trait asks that the library's contract doesn't give through a
  dictionary (#2285), `Crossing::FactBoundary` (#2556).
- `traits/dictionaries/placed.rs`' `placed_dictionary`: an entry of a bound
  that isn't an iterator's, taking or giving a value generic code holds in
  a place inside another value. A new cell, `Crossing::PlacedDictionary`,
  lifted by #2571: #2255, the issue its words name, is closed with it in
  place, and no two cells share an issue.
- `iterators/hint_asks.rs`' `dictionary_grown`: a bound's `extend` of a
  std collection whose items' size is a type parameter's, in a crate that
  reads a capacity, `Crossing::HintArgument` (#2553).

The scan's others aren't of the kind and stay as they are: what
`traits/dictionaries.rs` and `std_types/io.rs` refuse of #1302, and what
`iterators/mir_steps/closing.rs`, `iterators/stepping.rs` and
`traits/impls/defaults.rs` refuse, none of which a crossing's conversion
would lift. There are now 22 cells and 52 refusals.
