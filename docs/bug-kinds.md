# Bug kinds and areas

Kinds exist to signal refactoring: one bug is fixed where it is, but many
of one kind in one area say that the area's structure keeps making them
([ADR 1085](decisions/1085-issues-carry-a-kind-and-an-area.md)).

Every issue carries one priority label ([CONTRIBUTING.md](../CONTRIBUTING.md#issue-priority)),
one `kind: …`, one `area: …`, and `regression` where it applies. This page
is the one authority on kinds, areas and what a filer records. A
`Nightly red: <date>` report lists failures rather than one bug: it carries
none of these, and each bug it finds is filed as an issue of its own.

## Kinds

Ask the questions in order. The first "yes" is the issue's kind. A later
"yes" is a secondary kind, said in the issue's text, not labelled. The
questions ask what the fix had to change.

| # | Label | Definition | Ask | Examples | A cluster calls for | Violates ([AGENTS.md](../AGENTS.md)) |
|---|---|---|---|---|---|---|
| 1 | `kind: wrong decision` | The code did what an ADR or guide page prescribed, and that decision changes results or rests on a false premise | Did the code behave as a written decision said, and did the fix amend that decision? | #554 (ADR 1006), #555 (ADR 1062), #538 (ADR 0280) | A gate on decisions: each names its ADR 0262 case and a test of its premise; audit the related ADRs | Correctness first; ADR 0262's "results change: preserve Rust's" |
| 2 | `kind: duplicated authority` | Two or more places answer one question, or one fact has two meanings, and they disagree | Did the fix delete or merge a decider, make one ask the other, or split an overloaded fact? | #539, #614, #549 | One owner, and an architecture test that bans the second decider | One way, one authority: define each rule once |
| 3 | `kind: reconstructed fact` | A decision infers a semantic fact from a proxy (JS shape, a name, a span, a type walk) instead of asking the fact's owner | Did the faulty decision read output form or a proxy where an owner, or MIR, knew the answer? | #466, #622, #450 | The fact carried explicitly, and the decision added to the architecture test's list of decisions read from JS shape | Never reconstruct decisions from names or JS shapes |
| 4 | `kind: representation gap` | The IR, a fact type or a value form can't express a needed distinction, so code approximated | Did the fix add a variant, type, fact or value form? | #470, #424 | A change of representation; consumers keep approximating until it lands | Be ambitious: change any representation |
| 5 | `kind: unsound optimization` | A rewrite or analysis that simplifies (fold, inline, move, substitute, rename, merge into `try`/`finally`, narrow) is right for its usual inputs, but its precondition misses an effect, getter, throw, alias or jump | Was the JS right before the rewrite or analysis changed it? | #463, #485, #515 | One effects model every rewrite asks, and a differential oracle, the stage on against off | Never trade correctness for readability |
| 6 | `kind: protocol gap` | One implementer of a runtime protocol with many (iterator steps, `unwind`, drops) lacks a method or doesn't pass a parameter on | Did the fix add or forward a method or parameter in one or more implementers? | #621, #626, #492 | The protocol defined once: a catalog, one forwarding combinator, a conformance matrix | One semantic owner and catalog per rule |
| 7 | `kind: enumerated cases` | A rule is a hand-kept list of names, statement kinds, helpers or declaration paths, and the new case isn't in it | Did the fix add an entry to a list or a match arm rather than change logic? | #602, #520, #522 | The list derived from its owner, or an exhaustive match with no `_ =>` default | Extend a general rule instead of adding exceptions |
| 8 | `kind: JS semantics hazard` | The JS would be right if objects were plain records, but getters, `this`, prototype keys, realms or missing globals make it wrong | Would the code be right on plain records? | #427, #430, #611 | The hazard modelled as a fact (`Known::Getter`, `Known::ThisUnread`), and a JS-side oracle | Preserve every observable detail; ADR 0262 |
| 9 | `kind: missing case` | The right owner, asked the right question, answers wrongly for one input shape: a type parameter, a library's optimized MIR, an opaque type, a char or BigInt, an unwind edge, a subclass | Did the fix extend one owner's computation to a new shape? | #472, #487, #496 | A coverage gap: the area's shapes generated as a matrix and compared against native Rust | Prove behavior against native Rust |
| 10 | `kind: local slip` | A wrong formula or condition in one place, with no other decider, list or representation involved | None of 1–9 applies, and the fix is a local change? | #417 | No refactor; many in one file is a coverage gap there | None |

**The fallback.** `kind: local slip` is only for an issue where questions
1–9 are all "no". If an owner, a proxy or a list was involved, the issue
has that kind, even where the fix was one line: #420's `$charStep`
existed and wasn't used, so it is a `missing case`. Many local slips mean
filers stopped asking too early.

## Areas

The area is the owner whose question was answered wrongly, as
[the architecture](architecture.md)'s tables name it, not the file where
the wrong JS shows: tidy's looped drops (`mir/tidy/unwinds.rs`) are
`drops-unwind`, since where a drop runs is the drops owner's question.
Paths are `src/lower/`'s, except those starting with `src/`, `tooling/`
or `typescript/`.

| Label | Its questions | Owners |
|---|---|---|
| `area: iterators` | Iterators, adapters and combinators, std's compiled iterator structs, and the steps, drops and unwinding they pass on | `iterators.rs`, `iterators/`, `combinators.rs`, `combinators/`, `mir/iter/`, `recognition/iterators.rs`, `recognition/own_steps.rs`, `recognition/slice_steps.rs`, `recognition/specialization.rs`, `recognition/owners.rs`, `analysis/iterator_steps.rs`; `src/runtime/`'s step helpers (`iterator.js`, `steps.js`, `back_steps.js`, `closing.js`, …) |
| `area: mut-refs` | A `&mut`'s form (the place, a box, a handle, an in-place cell), copy-back, copies, cells and `RefCell` borrows, pointers | `mut_refs.rs`, `pointers.rs`, `mir/local_facts.rs` (`place_reference`), `representation.rs` (`is_cell_pointee`, `property_cell`), `copies.rs`, `std_types/cell.rs`, `drops/ref_cells.rs`, `analysis/cell_borrows.rs`, `analysis/cell_flow.rs` |
| `area: nullability` | How a `None` is held, `undefined`, `null`, an absent key or a box, and who may trust which | `mir/nulled.rs`, `mir/patterns.rs` (`null_nones`), `std_types/option.rs`, `representation/options.rs`, `mir/nested_options.rs`, `bindings/nullable.rs`, `analysis/nulled_params.rs`, `src/js/make.rs` (`presence`), `src/js/storage.rs` (`Storage`), `src/js/undefined_keys.rs` |
| `area: drops-unwind` | Destructors, drop glue and shims, where a drop runs, and what runs as a panic unwinds | `drops.rs`, `drops/`, `mir/drops.rs`, `analysis/unwinding.rs`, `traits/dyns.rs` (a `dyn FnOnce`'s drop) |
| `area: readability-passes` | Rewrites that make the JS read as a person writes it, and whether a read may move, repeat, be substituted or go; where a comment goes | `mir/tidy.rs`, `mir/tidy/`, `mir/temporaries.rs`, `body_queries/ranges/narrowing.rs`, `src/prepare.rs`, `src/prepare/`, `src/js/make.rs` (inlining), `src/js/expr_walk.rs` (`may_throw`), `src/to_oxc/starts.rs`, `src/to_oxc/trailing.rs` |
| `area: modules-linking` | Modules, imports, re-exports and specifiers, what a module's load makes and in what order, output paths, packages, the manifest and hosts | `reexports.rs`, `default_exports.rs`, `load_order.rs`, `analysis/lazy_locals.rs`, `pipeline/load_cycles.rs`, `bindings/held.rs`; `src/link.rs`, `src/output.rs`, `src/packages.rs`, `src/library.rs`, `src/manifest.rs`, `src/to_oxc/loads.rs`, `src/to_oxc/header.rs`, `tooling/` |
| `area: declarations` | The `.d.ts`: what each export's TypeScript says | `declarations.rs`, `declarations/`, `src/typescript.rs`, `typescript/` |
| `area: functions-calls` | Which JS function a use of a fn gives, and with which extra parameters; calls, fn pointers, closures, trait dispatch and evidence | `items.rs`, `calls.rs`, `mir/calls.rs`, `mir/dispatch.rs`, `passed_on.rs`, `binding_calls.rs`, `instances.rs`, `traits.rs`, `traits/`, `parameter_names.rs`, `analysis/fn_pointers.rs`, `analysis/scopes.rs`, `analysis/js_given.rs` |
| `area: classes-js-objects` | JS's object model: classes, objects of methods, namespaces, prototypes, field keys, getters and `this`; untagged values; serde's objects; JSX | `classes.rs`, `classes/`, `untagged.rs`, `untagged/`, `bindings.rs` (`field_key`), `field_keys.rs`, `serde.rs`, `jsx.rs`, `src/js/stmt_walk.rs` (`reads_this`) |
| `area: ranges-numbers` | std's scalar values: integers, floats, chars, strings and ranges, their steps, bounds and formatting | `std_types/number/`, `std_types/text.rs`, `std_types/range.rs`, `mir/offsets.rs`, `analysis/wholes.rs`, `bindings/whole.rs`, `body_queries/ranges/`, `display.rs`, `display/`, `format_args.rs`, `format_spec.rs`; `src/runtime/`'s number and range helpers |

An owner in no row takes the area of the row nearest it. There are at most
ten areas. Where a count shows many issues fitting none, a change to the
areas amends ADR 1085; it doesn't add an eleventh.

## Regression

`regression` marks an issue where something worked, and a known commit, its
first bad commit, broke it: bisected, or stated by the issue or the fix. A
bug that never worked since its feature landed (`born:<SHA>`) or whose
origin isn't known (`old`) has no `regression` label.

## Filing

Record these in the issue, so its kind can be checked
([.github/ISSUE_TEMPLATE/bug.md](../.github/ISSUE_TEMPLATE/bug.md)):

1. **Where the wrong JS is decided**, not where it shows: the file and
   function, and the phase.
2. **The owner** of that question in the architecture's
   [semantic owners](architecture.md#semantic-owners), and whether the
   faulty code asked it. If another place decides the same thing, name it:
   file and function.
3. **Regression**: the first bad commit (bisected), or `born:<SHA>` where it
   never worked since the feature, or `old`. Say what kind of change the
   first bad commit was: readability, a correctness fix (#N) or a feature.
4. **How it was found**: port (rd number), generated program (run and
   seed), mutation (name), corpus or CI test (name), probe or matrix
   (which), review, or external.
5. **The ADR 0262 case**, and whether native comparison can see it. If it
   can't, name the JS-side test that holds it.
6. **The kind**, with one line answering its question.

**The filer proposes the kind and the area, and the fixer confirms or
corrects them at close**, from what the fix changed, saying so in the
closing comment where they change.

## Reading the signal

Count kind × area periodically, weekly and before choosing the next
refactor:

```sh
gh issue list --state all --limit 2000 --json labels --jq '[.[] | [.labels[].name] | {k: (map(select(startswith("kind: ")))[0]), a: (map(select(startswith("area: ")))[0])} | select(.k != null and .a != null)] | group_by([.a, .k]) | sort_by(-length) | .[] | "\(length)\t\(.[0].a)\t\(.[0].k)"'
```

Three or more of one kind in one area is a cluster. The kind's "A cluster
calls for" column says what change it needs, made in that area's owners. A
cluster that keeps growing after its fixes says those fixes patched
instances, not the structure. A cluster of `regression`s in one area says
the changes made there need an oracle before they land.

The first count was of the 83 P0s of 2026-10-29 to 2026-11-04: `iterators`
× `protocol gap` 10, `readability-passes` × `unsound optimization` 6,
`mut-refs` × `missing case` 6 and `declarations` × `duplicated authority`
5, and 12 of the 16 regressions came from readability changes.
[ADR 1085](decisions/1085-issues-carry-a-kind-and-an-area.md#what-the-first-count-calls-for)
ranks the structural changes they call for.
