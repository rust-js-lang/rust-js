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
4. `module_fns`, each module's function of a recursive type's `==`, clone
   or order, and its dictionaries' and instances' values, is
   `Settled::module_fns` for those the bodies before made. What a body
   makes and calls is its own (`FnCx::module_fns_made`, a `MadeHere`): its
   lookups are of both, and each one it makes counts both as made before
   it, `made_before`, as the module writes them in that order. Its calls
   are kept in order, each with the one it was made in, and the pipeline
   settles them after the body (`ModuleFns::settle`), replaying them as
   the one cell did, so who calls what, and so where each is written, is
   as before. Nothing a body reads is a call's: only the pipeline's
   `written_before` and the drops' settling read those. The names a body
   may not take are asked, `FnCx::module_fn_names`, of both.
   test/architecture-state.test.ts holds `module_fns_made` to
   recursive_fns.rs. Output unchanged. Mutations module-fn-made-before-unseen
   and module-fns-made-unsettled are new, and three are retargeted, each
   caught.
5. `nulled_params`' three `RefCell`s, what lowering saw of the calls of
   the crate's functions with own nullable parameters, the parameters
   found `null` or a value at every call, and the functions found to
   return so (ADR 0861, amended, rd613, rd632), are `Settled::nulled`, a
   `NulledFound`. `NulledParams` keeps only what its scan found before any
   body, and holds no cell. A body's own sightings are `FnCx::nulled_seen`,
   a `NulledSeen`: each call it lowers (`nulled_call_seen`, the one writer,
   mir/nulled.rs), each function it finds returns `null` or a value, and
   each it asks of that isn't found yet. Its asks read what's settled and
   its own, so a closure found so by its maker's body is seen there, as
   before, with no round more. The pipeline settles it after the body, and
   `again` reads and writes what's settled, as it read the cells, so the
   rounds are as before. test/architecture-state.test.ts holds
   `nulled_seen` to mir/nulled.rs. Output unchanged. Mutation
   nulled-seen-unsettled is new, and six are retargeted, each caught. The
   check of a body's own found returns saves a lowering again, and changes
   no output, so no mutation holds it.
6. `ref_cells` and `own_callbacks`, the two `OnceCell`s, are each found of
   the crate's facts alone, never of what a body found. Measured with the
   release compiler on the Mac, each found eagerly in a context of the
   crate before the first body, over the corpus's 120 largest programs in
   4 parts, 3 rounds, at a load of 6 to 10 (`uptime`, 1 min), no quiet
   machine to be had:

   | Fact | Of the compile |
   |---|---|
   | `own_callbacks` | 3.3 to 3.5 ms of 12.9 to 16.0 s, 0.02% to 0.03% |
   | `ref_cells` | 257 to 267 ms, 1.6% to 2.1%; 3.6% to 3.9% of the worst programs, `io_buf_reader`'s, `owner_handle_quiet`'s |

   So `own_callbacks`, under 1%, is a fact found before any body
   (`contexts::own_callbacks`, in a context of the crate's first
   function). `CrateFacts` is made once without it, then again with it.
   `ref_cells`, over 1%, is a declared memo, `crate_facts::Lazy`: found the
   first time a body asks, the one memo `CrateFacts` may hold.
   test/architecture-observations.test.ts holds that `CrateFacts` has no
   cell left and one `Lazy`, `ref_cells`, found only by
   drops/ref_cells.rs. Output unchanged. Mutation own-callback-params-none
   is retargeted onto its finder, in `lower/pipeline/contexts.ts`, and
   caught.
7. The rounds that settle what lowering finds are one bounded loop
   (`pipeline/settle.rs`). Once each body queued is lowered, a protocol may
   ask for more. There are three, asked in order: the codecs something
   used, the own nullable parameters each call gives `null` or a value,
   and the bodies a load cycle reads, once (`load_cycles::again`, split
   from pipeline.rs first). Each round is one protocol's, counted by
   `settle::Rounds`, and a round that asks for nothing more ends them.
   The bound comes from what was measured. Over the corpus's 2,523 cases,
   each in both builds, plus the crate pairs and triples and the serde
   suites, the most rounds any compile took of one protocol was 2:
   `nulled_returns_given_on`'s of its nullable parameters, and a serde
   suite's of its codecs. A load cycle takes 1. The bound is 8 times that,
   16 rounds a protocol. The codecs may also take as many rounds as the
   crate has codecs, as each round lowers at least one more and never one
   again, so a type in a type in a type takes a round each. A protocol past
   its bound is refused as rust-js's own bug, naming the protocol: "rust-js:
   the own nullable parameters each call gives `null` or a value didn't
   settle in 16 rounds of lowering the crate again: a bug of rust-js's".
   Unit tests hold that a round that changes nothing stops, the first one
   too; that a protocol that never settles is refused by name at its
   bound; and that the codecs take a round each. Output unchanged.
   Mutations settle-unbounded and settle-codecs-bounded-as-the-others
   (`lower/pipeline/settle.ts`) are caught by them.

M2 is done: `CrateFacts` holds no cell, and one `Lazy`; what a body finds
is given back; and `Settled` is the pipeline's alone, settled in bounded
rounds.

## M3: one call graph

1. The crate's call graph is built once, in analysis.rs
   (`call_graph::crate_graph`). It covers each function's MIR the emitter
   reads, and each constant's MIR rustc evaluates it by (a `const`'s, a
   `static`'s, an inline `const { .. }`'s), whose calls are a function's
   callers too: m3-max's const callers (#1742), folded in from
   `intervals::given_parameters`, which now asks the shared graph.
   Mutation intervals-constants-call-nothing follows its code into
   `lower/call_graph.ts`, and is caught.
2. The throwing functions (ADR 0644) are found over the graph's
   components, each after those it calls, each to a fixed point from none
   of its own. Whether a function is one depends only on its callees, so
   this is the least fixed point the whole-crate rounds found, in one pass.
   Output unchanged. test/throwing-calls.test.ts gains one whose result is
   another throwing function's call. Mutation throwing-fn-before-its-callees,
   the components walked callers first, is caught by it;
   throwing-fn-named-as-a-value is retargeted, and caught.
3. The graph holds each body's names beside its calls, `Crate::names_of`:
   each function the body calls or names as a value, of its generic
   arguments there, and where, as its THIR names it, or of a body of MIR
   alone (a library's the crate compiles, ADR 0914) its MIR. This was
   `instances::calls_of`, a scan each analysis ran of its own. The five
   analyses that ran it can't ask the graph's calls instead: those are
   MIR's terminators, resolved, with no generic arguments and no function
   named as a value, where these ask which functions the source names and
   of what types. So one owner, call_graph.rs, has two tables: who calls
   whom at run time, and who names whom as written. The scan is
   `call_graph::names`; the graph runs it once of each body the analyses
   read, and is built before the first of them, `drop_params`. The
   formatting failures (`fmt_failures::failing_fns`) ask `names_of` first.
   Output unchanged. Mutation closure-constant-functions-uncalled follows
   its code into `lower/call_graph.ts`; fmt-failing-call-of-mir-unseen is
   retargeted. Both are caught.
4. The rest of the analyses ask `names_of` too, one a commit, each
   unchanged in output: the type parameters given drops
   (`drops::drop_params`), whether a `dyn Error` drops
   (`closures::error_dyns_drop`), what prints and what's written into a
   writer (`prints::prints`, `shown_by`, `prints_to`; `shown_by` scanned
   each writer again on each round of its fixed point), and the parameters
   written into a writer (`written_params::written_params`). Their loops'
   mutations are caught; instance-writer-prints-unseen and
   written-into-writers-unseen are retargeted.
5. The names are scanned once, and the instances' callers ask them too.
   `call_graph::Names::of` makes the table with what the analysis stands
   on (ground.rs). That is before the bodies are told which adapters each
   makes in place, which changes nothing a body names, and before the
   clones ask the callers' carrier (ADR 1115). The graph then holds the
   table. `instances::given_calls_of`, the carrier `callers_of` and
   `calls_by_caller` ask `names_of`, where `given_calls_of` had scanned
   each body again for each of the seven analyses that built the carrier.
   The scan, `named_by`, is private to call_graph.rs, and `Names::of` is
   its one caller. Output unchanged. Mutation
   generic-clones-unseen-by-rc-counts is retargeted; it and
   js-alone-given-counted are caught.

test/architecture-calls.test.ts holds the graph built once and asked. It
also holds the scan private to the graph and run once, from ground.rs.
