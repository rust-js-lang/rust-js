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
