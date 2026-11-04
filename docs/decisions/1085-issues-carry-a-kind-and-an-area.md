# 1085. Issues carry a kind and an area

Status: Accepted (2026-11-04, the user's decision). Extends
[CONTRIBUTING.md's issue priority](../../CONTRIBUTING.md#issue-priority):
an issue's priority says how bad it is, and its kind and area say what to
restructure.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: how issues are
filed, no difference between Rust and JS.

## Context

From 2026-10-29 to 2026-11-04, 83 issues were labelled `P0: wrong answer`,
74 closed and 9 open. Each was read with its comments, its fixing commit's
message and the files the fix changed. The findings:

- **Most are structural.** 76 of the 83 fall into 11 families of three or
  more issues that share a mechanism, and only about 8 are true one-offs.
  The largest are the iterator step, drop and unwind protocol (21), one
  `&mut` with several JS forms (12) and the readability rewrites' read
  count and order (9).
- **12 of the 16 regressions came from readability changes**: fewer `try`
  blocks (#440, #463, #465), `=== null` without `?? null` (#519, #539,
  #541), `Map<K, V | null>` (#443), folded let chains (#485), inlined
  arrows (#462), a module variable read in place of its copy (#515), a
  whole write to the place (#466) and `export * from` (#602). The other
  four came from another P0's fix.
- **The fixes mostly patched instances.** In the iterator family, nine
  issues of the same mechanism were filed after its fixes, all on
  2026-11-04, and seven are still open (#621–#627).

A priority label says how bad an issue is, not what kind of mistake it is
or whose. Nothing recorded the mechanism, so seeing that these were a few
structural problems took reading all 83 issues and their fixes again.

## Decision

**Every issue carries one priority, one `kind: …`, one `area: …`, and
`regression` where a known commit broke what worked.**
[docs/bug-kinds.md](../bug-kinds.md) is the one authority on them:

- **Ten kinds**, each with a question about what the fix had to change,
  asked in order. The first "yes" is the kind: `wrong decision`,
  `duplicated authority`, `reconstructed fact`, `representation gap`,
  `unsound optimization`, `protocol gap`, `enumerated cases`, `JS
  semantics hazard`, `missing case`, and `local slip` where none of the
  nine others applies.
- **Ten areas**, each a set of owners in [the architecture](../architecture.md):
  `iterators`, `mut-refs`, `nullability`, `drops-unwind`,
  `readability-passes`, `modules-linking`, `declarations`,
  `functions-calls`, `classes-js-objects`, `ranges-numbers`. The area is
  the owner whose question was answered wrongly, not where the JS shows.
- **`regression`** where the first bad commit is known. One that never
  worked since its feature, or whose origin isn't known, isn't one.
- **The filer records six facts** (where the JS is decided, the owner, the
  regression, how it was found, the ADR 0262 case, the kind) and proposes
  the kind and area. The fixer confirms or corrects them at close, from
  what the fix changed.
- **A periodic kind × area count** picks the next refactor: a cluster of
  three or more of one kind in one area calls for the change its kind
  names, in that area's owners.

## Why

- **The kind is the refactor.** Each kind names the structural change a
  cluster of it needs: a duplicated authority needs one owner, a protocol
  gap a catalog and a conformance matrix, an unsound optimization an
  effects model and a stage differential. A family found by reading
  issues is what the count finds from labels.
- **The area names where.** `missing case` alone spans ten families of
  the 83. Kind × area separates them: `iterators` × `protocol gap` is 10,
  `readability-passes` × `unsound optimization` 6, `declarations` ×
  `duplicated authority` 5. Areas are owners, so a cluster names the
  modules to restructure.
- **Ordered questions make the kind decidable.** Each asks what the fix
  changed, which the fixer knows, so it is checked at close. Tried on the
  83, the fallback took one issue (#417), and about a dozen fit poorly but
  had a clear first "yes" (#422, #424, #449, #478, #527, #541, #564).
- **`regression` points at the changes that need oracles.** 16 of the 83
  had a known first bad commit, and 12 of those came from readability
  changes: a cluster that says those changes need a differential before
  they land.

## Alternatives

- **The families as labels** (F-ITER, F-REPR and so on). They name today's
  mechanisms, would grow with each new one, and stop meaning anything once
  their structure is fixed. Kinds are mechanisms in general, and areas are
  owners that last.
- **Several kinds per issue.** Most issues answer more than one question
  yes, so the count would be noise. The first "yes" is the label; a
  secondary kind goes in the text.
- **The phase as the second axis.** 41 of the 83 were "lowering", which
  names no refactor. An owner does.
- **More kinds**, "build input mismatch" (#424) or "unowned invariant"
  (#541). The first fit one issue; the second overlapped `duplicated
  authority` too much.
- **No labels, and an analysis like this one each period.** It costs
  reading every issue again, and its families aren't comparable from one
  period to the next.

## Consequences

- The 21 labels are on GitHub, each with a one-line description, and on
  the 83 P0s and every issue open on 2026-11-04. The P0s' kinds are
  those of the classification the count below was made from.
- CONTRIBUTING.md's issue priority links docs/bug-kinds.md, and
  `.github/ISSUE_TEMPLATE/bug.md` asks for the six facts.
- docs/architecture.md's semantic owners gained the two owners the areas
  name that it lacked: `recognition/own_steps.rs`'s own-step questions
  and `js/undefined_keys.rs`'s keys left out.
- A Nightly-red report carries no kind or area. Each bug it finds is filed
  as an issue of its own.
- There are at most ten areas. A count that shows many issues fitting none
  amends this decision to change the areas, rather than adding an eleventh.

## What the first count calls for

The kind × area cells of three or more, of the 83 P0s:

| Area × kind | n |
|---|---:|
| `iterators` × `protocol gap` | 10 |
| `readability-passes` × `unsound optimization` | 6 |
| `mut-refs` × `missing case` | 6 |
| `declarations` × `duplicated authority` | 5 |
| `iterators` × `missing case` | 4 |
| `iterators` × `reconstructed fact` | 3 |
| `drops-unwind` × `unsound optimization` | 3 |
| `drops-unwind` × `missing case` | 3 |
| `modules-linking` × `duplicated authority` | 3 |
| `functions-calls` × `missing case` | 3 |
| `ranges-numbers` × `missing case` | 3 |

The structural changes they call for, ranked by what they would have
prevented. S is about 1–2 days, M about a week and L two or more, for
one agent:

| # | Change | Size | Would have prevented |
|---|---|---|---|
| 1 | An iterator conformance matrix against native Rust in both builds (plain and `--std-iterators`), generated: in full nightly, a path-filtered sample per push | M | all 21 of the iterator family, and #547, #548 |
| 2 | One catalog of `Iterator`'s overridable steps, and one runtime forwarding combinator, so adapters stop forwarding by hand and `$given` and `$iterator` read one table | L | #426 #492 #494 #505 #520 #521 #522 #621 #623 #625 #626 |
| 3 | A readability-pass oracle: tidy's stage differential extended to `prepare` and generated programs, a counting-getter JS oracle, and one effects API every rewrite asks | M | #430 #440 #452 #457 #462 #463 #465 #482 #485 #515 #525 #564 |
| 4 | One owner of a `&mut`'s form, its operations exhaustive methods, an architecture test on the form predicates, and a `&mut` corpus matrix | L | #466 #469 #472 #478 #482 #487 #527 #528 #536 #614 #619 |
| 5 | One `JsSignature` from lowering, which every `.d.ts` path and the function-form choice read, and a generated TypeScript-caller test | M | #529 #549 #551 #559 #560 #565 #596 #619 #620 |
| 6 | One owner of a place's `None` encoding over its writers and JS entry points, and a JS-boundary test of what each `.d.ts` permits | M | #443 #470 #471 #519 #539 #541, partly #538 #575 #611 |
| 7 | An owner query "may stepping this type unwind", replacing `chain_may_throw` and `panicking_steps`, on the architecture test's list of decisions read from JS shape | S | #622 #624 #627 |
| 8 | An ADR 0262 gate: each ADR names its case and a test of its premise, and the ADRs that chose result-changing behavior (1006, 0280, 1062, 0465) are audited | S | #422 #538 #554 #555 #558 #575 |
| 9 | Drops moved only where the drops owner's hint allows, in `mir/tidy/unwinds.rs` | S–M | #440 #463 #465 |
| 10 | Every module load symbolic until output planning, relinked exhaustively, and a layout matrix (mostly landed in e0d5b7077 and 44b3e851f) | S | #572 #592 #602 #618 |
| 11 | Boundary matrices for ranges and fused floats, with stale exclusions removed | S | #414 #417 #420 #421 |
| 12 | rustc's drop-glue and once-shim MIR lowered instead of hand-written versions, if its MIR is reachable | M–L | #409 #416 |

Changes 1 and 3 are oracles: they automate the hand probes that found 35
of the 83 and the ports that found 28. Changes 2, 4, 5 and 6 are the
ownership refactors that stop each family from recurring.
