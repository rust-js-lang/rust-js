# 1148. Lowering gives back what it found; the crate's facts are read alike by each body

Status: Accepted (2026-11-14, #1671's change 4, M2 to M4, the plan the lead
approved; in progress, a step a commit, each recorded here).

Case ([0262](0262-when-rust-and-js-disagree.md)): none. This is about how
rust-js's lowering is put together. No output changes.

## Context

Review #1671's change 4 found that `CrateFacts`, documented as the
analysis's immutable inputs, held cells that body lowering writes:

- `asked`, the library items a body found it compiles itself;
- `loads_in_cycles`;
- `module_fns`, a module's functions of a recursive type;
- `field_reads`;
- `ref_cells` and `own_callbacks`, found once and kept.

The analysis `nulled_params` kept three more cells of its own.

What one body wrote, a later one read, and the pipeline lowered bodies
again by four protocols of its own:

- `Lowered::Instances`;
- `nulled_params.again`;
- the `loads_in_cycles` round;
- `Lowered::MadeWhereUsed`.

Nothing said which reads were of facts settled before any body and which
of what earlier bodies happened to find.

## Decision

**`CrateFacts` is what's found before any body is lowered, and holds no
cell.** Each cell moves to one of two places:

- **`Settled`** holds what the bodies lowered before settled, which a body
  lowered after reads. Only the pipeline writes it, between one body and
  the next. A body holds it by a shared reference (`FnCx::settled`), given
  beside the shared context (`contexts::fn_cx((&shared, &settled), ..)`).
- **What a body gives back.** What a body finds is returned with its
  `LoweredFn`, or gathered from it, as `drops_observed` already is, and the
  pipeline settles it.

The pipeline's re-run protocols become one explicit loop, `settle()`. Its
bound is explicit, and a test holds that a round which changes nothing
stops.

test/architecture-observations.test.ts holds the rules:

- `CrateFacts`' cells are a list that only shrinks, emptied by M2's last
  step;
- only `pipeline.rs` writes `Settled`;
- a body holds `Settled` by a shared reference.

M3 (one call graph) and M4 (each fact declared `Local`, `Summary` or
`ClosedWorld`) are recorded here as they land.

## Steps

1. `field_reads`, each untagged enum's method that reads each variant's
   field, lowered before its callers (rd317), is `Settled::field_reads`.
   The pipeline inserts it once the method is lowered. Mutation
   field-read-unsettled is caught.
2. `loads_in_cycles`, each body run where its module loads that reads a
   `const` of a module that may load after its own, is
   `Settled::loads_in_cycles`. The pipeline extends it once a round's
   load cycles are known. The mutations load-reads-by-name and
   load-reads-own-module-by-value are retargeted, and caught.
3. `asked`, the library items a body found the crate compiles itself, is
   the body's own (`FnCx::asked_of_libraries`). The pipeline gathers it after each
   body, and after the library's `RefCell`s context, as it gathers
   `drops_observed`. Mutation asked-ungathered is caught. Two of the three
   asks still gave back JS's `undefined` rather than ADR 1147's hole: a
   library's impl that isn't its dictionary's entry, and a library's trait
   of classes' default. Both are `Hole::Asked` now, and
   test/architecture-holes.test.ts holds every ask to give one.
