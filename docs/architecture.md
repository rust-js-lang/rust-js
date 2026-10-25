# Architecture

The current map of rust-js: responsibilities, contracts, and where a change
belongs. [AGENTS.md](../AGENTS.md) defines the principles;
[the guide](guide/README.md) explains the supported language and APIs;
[design decisions](decisions/README.md) record why. This page describes the code,
not a proposed architecture.

[Architecture tests](../test/architecture.test.ts) enforce layer dependencies,
selected ownership boundaries, module coverage, and the existence of the
owner APIs below. They do not prove every architectural or semantic claim.
`bun run architecture` reports dependencies and ownership; behavior tests
check what compiled programs observe.

## Policies

- **One pinned toolchain.** [rust-toolchain.toml](../rust-toolchain.toml)
  owns the exact Rust version and target components. rustc owns Rust validity;
  rust-js uses its internal THIR and MIR APIs. Upgrades require adapting the
  compiler and rebuilding matching native dependencies and WASI artifacts.
  `src/command.rs::TARGET` fixes the checked target to `wasm32-unknown-unknown`,
  including 32-bit `usize`, independently of the host running the compiler.
- **Modern JavaScript.** Output is readable JS/JSX and ES modules. There is
  no ES-version selection or downlevel pass. Application tooling owns JSX
  transformation, bundling, deployment targets and polyfills; rust-js owns
  Rust semantics and source maps. Syntax lowering alone cannot supply runtime
  capabilities such as BigInt. A numbered compatibility matrix remains open.
- **Single semantic authority.** Each question has one owner. Other phases
  call its API or consume its explicit result, including through manifests.
  Owners are distributed by responsibility; no phase reconstructs another's
  rule from generated text or private representation details.

## Where the code is

The native compiler and the WASI build share `src/main.rs` and its modules.
`wasm/Cargo.toml` builds the same sources against a patched rustc frontend
compiled for WASI; it is a separate build package, not a second compiler.

| Responsibility | Code |
| --- | --- |
| Compiler entry, command parsing, Cargo wrapper, panic reporting | `src/main.rs`, `src/command.rs`, `src/cargo.rs`, `src/panic_runtime.rs` |
| JSX syntax expansion and provenance | `src/jsx_syntax.rs`, `src/jsx_syntax/` |
| Rust analysis and lowering | `src/lower.rs`, `src/lower/` (detailed below) |
| Owned JS tree, source arena and comments | `src/js.rs`, `src/js/`, `src/program.rs`, `src/comments.rs` |
| Reachability, linking and names | `src/reachability.rs`, `src/link.rs`, `src/names.rs` |
| Readability and printing | `src/prepare.rs`, `src/prepare/`, `src/to_oxc.rs`, `src/to_oxc/`, `src/format.rs`, `src/typescript.rs`, `src/prettier.rs`; `typescript/` prints the declaration model |
| Library contracts, configuration, hooks and path utilities | `src/library.rs`, `src/settings.rs`, `src/hooks.rs`, `src/paths.rs` |
| Artifact planning, manifest and publication | `src/output.rs`, `src/manifest.rs`, `src/publish.rs` |
| Runtime helper catalog and implementations | `src/runtime.rs`, `src/runtime/`; generated package in `runtime/` |
| Compiled standard-library bodies | `std/lib.rs`, generated `std/*.js` and `std/manifest.json` |
| Native app build and editor check | `tooling/cli.js`, `tooling/build.js`, `tooling/cargo.js`, `tooling/check.js` |
| Host manifest validation and file publication | `tooling/manifest.js`, `tooling/publish.js` |
| npm/Cargo integration and shipped binding catalog | `tooling/patch.js`, `tooling/crates.js` |
| Shipped JS, web, React, Node and Next.js bindings | `builtins/`, `webapi/`, `react/`, `node/`, `next/`; the authoritative list is `tooling/crates.js` |
| External crates used by compiler tests | `serde/` builds crates.io dependencies; it is not a shipped binding crate |
| WASI compiler build | `wasm/Cargo.toml`, `wasm/build.sh`, `wasm/patches/` |
| Playground UI and in-browser compiler host | `wasm/web/rust/`, `wasm/web/compiler-client.js`, `wasm/web/compiler-worker.js`, `wasm/web/frame-report.js` |
| Playground build and served assets | `wasm/web/compile-rust.ts`, `wasm/web/site.ts`, `wasm/web/vite.config.ts` |
| Tests and mutation tooling | `test/`, `scripts/mutations.ts`, `scripts/mutations/`, `scripts/matrix.ts` |
| Packaging, release and CI | `scripts/package-*.ts`, `scripts/ci.ts`, `.github/workflows/` |

## Hosts

An application compiles Rust through `rust-js build`, once per invocation
([ADR 0412](decisions/0412-rust-js-build-is-its-own-process.md)).
It writes JS/JSX and declarations beside the Rust; `--check` compares generated
source files without replacing them. Vite and Next.js consume those files.
They do not host a rust-js compiler plugin or schedule Rust compilation.

`tooling/build.js` finds the compiler, building a checkout's own from its
sources first ([ADR 0854](decisions/0854-a-checkouts-compiler-is-built-before-it-runs.md)), checks runtime package versions, and
coordinates Cargo and project checks. `tooling/cargo.js` reads Cargo metadata
and runs Cargo with rust-js as its wrapper; `src/cargo.rs` decides how each
invocation is handled. Workspace, path and npm-provided source crates compile
to separate JS libraries. Registry and Git dependencies are attempted too;
when compilation is refused, their Rust metadata and the refusal are retained
so use of unsupported code can produce a diagnostic. Binding crates and the
catalog `READ_AS_RUSTC_BUILDS_THEM` supply Rust metadata; the compiler lowers
their supported APIs through its binding and domain modules.
This supports dependency graphs within the compiler's supported subset, not
arbitrary crates or native-only behavior.

The playground has two distinct paths:

- **Building the playground itself:** `compile-rust.ts` runs the WASI compiler
  under Bun and publishes its output to disk through `tooling/publish.js`.
  `site.ts` prepares the compiler, sysroot, bindings and preview assets.
- **Compiling a user's program in the browser:** `compiler-client.js` creates
  one worker per compilation and owns timeout/cancellation. The worker calls
  `rust/compiler.rs`'s generated JS, which runs the WASI compiler in a fresh
  virtual filesystem and returns successful output described by its manifest.
  This path uses the supplied dependencies, not a general browser Cargo build.

The preview is a separate trust boundary: `rust/components/result_frame.rs`
uses an iframe with `sandbox="allow-scripts"`, and validates the sender and
run identity of messages. Compiler workers do not execute the resulting app.

## The pipeline

`main.rs` expands JSX during parsing and captures THIR before MIR construction
consumes it. It collects MIR and body facts, then requires rustc's full
analysis (including borrow checking and lints) to succeed before lowering.
Capturing a body is not permission to emit it.

```text
Rust source
    |
    v
rustc + JSX expansion -> captured THIR, MIR and body facts
    |
    v
rustc analysis succeeds
    |
    v
lower/pipeline.rs: analyze crate -> lower bodies -> assemble retained modules
    |                              MIR -> JS AST -> body tidy
    v
program::Unlinked             owned data: no rustc types beyond this boundary
    |
    v
link.rs -> link::Linked       resolve symbols and aliases; collect helpers
    |
    v
output.rs                    plan paths; prepare JS; validate; choose imports
    |                        print JS/maps and declarations; run transforms
    v
output::ArtifactPlan         all bytes and stale-file candidates prepared
    |
    v
publish.rs                   stage, replace, clean up; roll back on I/O error
```

Linking runs in the driver's rustc callback. Artifact planning waits until
rustc has exited successfully, including writing any requested Rust metadata.
Only the linker constructs `Linked`; only artifact planning constructs
`ArtifactPlan`. Compilation or planning failure prevents publication.
Project check hooks run after publication and may still fail the command.

| Phase | Owns | Must not do |
| --- | --- | --- |
| Syntax expansion | JSX-to-Rust syntax and source provenance | Bypass rustc or choose JS representations |
| Analysis | Validated bindings, indexes, names and crate/body facts | Emit functions or write output |
| Lowering pipeline | Scheduling, codecs, retention policy and module assembly | Resolve import aliases or publish |
| MIR and domain lowering | Evaluation order, control flow, places, copies, drops and representations | Format text or choose artifact paths |
| Body tidy | Readable control flow from JS and explicit lowering hints | Query rustc or mutable function context |
| Linker | Symbol resolution, collision-free import aliases and helper dependencies | Re-lower Rust or change expression semantics |
| Preparation | Readability rewrites on completed JS, preserving behavior | Depend on Rust types or predicted indentation |
| Printing | oxc conversion, JS layout and maps; TypeScript declaration printing | Decide Rust semantics, runtime imports or publication |
| Artifact planner | Paths, imports, prepared/printed bytes, transforms, manifest and stale candidates | Decide Rust semantics |
| Publisher | Staging, replacement, unchanged files, safe cleanup and rollback | Transform code or infer stale candidates |

### Dependency layers

These are dependency rules, not execution order. Paths are relative to `src/`.

| Layer | Modules | May depend on |
| --- | --- | --- |
| Driver | `main.rs`, `command.rs`, `cargo.rs`, `panic_runtime.rs` | Every layer |
| Front end | `lower.rs`, `lower/`, `jsx_syntax.rs`, `jsx_syntax/` | Front end and earlier-use owned data/utilities; no linking, preparation or publication |
| Owned output | `js.rs`, `js/`, `program.rs`, `comments.rs`, `link.rs`, `reachability.rs`, `names.rs`, `prepare.rs`, `prepare/`, `output.rs`, `publish.rs`, `manifest.rs`, `library.rs`, `runtime.rs`, `settings.rs`, `hooks.rs`, `paths.rs` | Owned output; no rustc |
| Printing | `to_oxc.rs`, `to_oxc/`, `format.rs`, `typescript.rs`, `prettier.rs` | Printing and owned output; no rustc |

The named orchestration exceptions are `output.rs`, which invokes the printing
adapters, and `hooks.rs`, which invokes `format.rs`. Only printing adapters use
oxc. `output.rs` also writes manifest JSON and the test runner's imports/calls.
The architecture test checks top-level dependency cycles; it treats the parts
of `lower/` as one module, not as individually cycle-checked units.

## Inside the front end

Paths in this section are relative to `src/lower/` unless stated otherwise.

State has three scopes: crate facts shared across functions; `FnCx` for a
function's context; and `mir::State` for one body's temporaries, labels,
captures and drop flags. `bodies.rs` manages nested-body entry and restoration.
Concern-specific state (drops, trait evidence, formatting, cloning and type
walks) is accessed only by the owners allowed in the architecture tests.

Recognition and query modules answer questions without emitting code.
Recognition owns std names, diagnostic identities and library-layout
assumptions; rustc lang items remain available through rustc's identity API.
`shortcuts.rs` only forwards recognition queries.

| Responsibility | Modules |
| --- | --- |
| Crate facts and validation | `analysis.rs`, `analysis/` |
| Body facts and integer ranges | `body_queries.rs`, `body_queries/`, `body_queries/ranges/` |
| std recognition and coverage | `recognition.rs`, `recognition/`, `shortcuts.rs`, `std_coverage.rs` |
| Function contexts, assembly and source capture | `bodies.rs`, `pipeline.rs`, `sources.rs`, `library.rs` |
| MIR lowering, control flow, calls, places, temporaries and drops | `mir.rs`, `mir/`, `mir/iter/` |
| Body readability passes | `mir/tidy.rs`, `mir/tidy/`, `mir/tidy/jumps/` |
| Value shape, copy policy, support and diagnostics | `representation.rs`, `copies.rs`, `support.rs`, `refusal.rs` |
| Values, patterns, calls and places | `aggregates.rs`, `literals.rs`, `patterns.rs`, `calls.rs`, `loops.rs`, `places.rs` |
| Mutable references, pointers, plain-data layout and unions | `mut_refs.rs`, `pointers.rs`, `plain.rs`, `unions.rs` |
| Destructors and their facts | `drops.rs`, `drops/`; `mir/drops.rs` follows rustc's drop elaboration |
| std types and calls | `std_types/`, `std_types/number/`; `std_types/values.rs` dispatches to each type's module |
| Iterators and combinators | `iterators.rs`, `iterators/`, `combinators.rs`, `combinators/` |
| Item references, traits, dictionaries and standard impls | `items.rs`, `traits.rs`, `traits/`, `std_impls.rs`, `ordering.rs` |
| Formatting | `display.rs`, `display/`, `format_args.rs`, `format_spec.rs` |
| Serde and supported external crate APIs | `serde.rs`, `serde/`, `shipped.rs` |
| JSX, JS bindings, untagged values and JS-visible uses | `jsx.rs`, `jsx_api.rs`, `bindings.rs`, `binding_calls.rs`, `untagged.rs`, `liveness.rs` |
| JS classes, `#[rust_js::class]` | `classes.rs` |
| Exported TypeScript declaration model, and the reader of `#[rust_js::types]` | `declarations.rs`, `declarations/` |

MIR determines execution order. `mir/cfg.rs` structures blocks;
`mir/temporaries.rs` folds temporary reads only when intervening effects allow
it. `mir/hints.rs` supplies source facts and `mir/offsets.rs` analyzes string
offsets without emission. `mir/tidy/stages.rs` lists the body rewrites, in
stages that each say what they leave a body with; `mir/tidy.rs` runs them and,
in a debug build, checks each stage's contracts (`mir/tidy/contracts.rs`).
`rust-js --tidy-until <stage/pass>` and `scripts/tidy-trace.ts` show what each
pass does to a crate's JS ([ADR 0659](decisions/0659-tidy-is-a-list-of-stages-with-contracts.md)).
`mir/tidy/moves.rs` decides where a temporary's value may be read in its
place, and why, apart from the rewrite that moves it ([ADR 0689](decisions/0689-a-moves-decision-is-apart-from-the-move.md)). Type-specific
rewrites remain with their type's owner; downstream `src/prepare.rs` works
only on the completed JS tree. The printer owns spelling and layout, including
expression forms determined while converting to oxc; Rust decisions stay upstream.

### Semantic owners

Consumers use these APIs instead of inspecting another owner's state or JS
representation. This is an index of shared contracts, not every function.
The architecture test checks the named modules and APIs exist and separately
enforces selected access rules; `bun run architecture` reports those boundaries.
In this table, `lower.rs`, `names.rs` and `runtime.rs` are at the `src/` root.

| Concept | Owner | What the others ask |
|---|---|---|
| MIR variable facts | `mir.rs` | `alike_local`, `plain_local`, `note_tested`, `note_falsy` |
| Drop behavior and placement | `drops.rs`, `drops/` | `drop_value`, `item_drop`, `drop_function`, `enter_body_drops` |
| JS iterator versus array representation | `iterators.rs` | `is_js_iterator`, `js_iterator`, `from_iterator`, `stepped`, `stepped_by`, `stepped_range` |
| Generic iterator evidence | `iterators.rs` | `given_as_it_is`, `iterator_dictionary`, `iterator_evidence`; which functions step one, `analysis/iterator_steps.rs` |
| Array stepping and lent iterators | `iterators.rs` | `stepped_items`, `is_stepped_items`, `iterated`, `lent_iterator`, `unlent` |
| Mutable-reference boxes and copy-back | `mut_refs.rs` | `boxed`, `copied_back`, `mut_borrowed`, `nested_mut_object` |
| Formatter output and MIR writer calls | `display.rs`; `mir/writer.rs` | `formatter_answer`, `written_of`, `builder_written`, `mir_writer_call` |
| Function and binding references | `items.rs` | `fn_ref`, `js_ref`, `resolve_instance` |
| Bindings held by modules | `bindings.rs`; `items.rs` | `held`, `held_path`; `binding_ref` |
| A module's default export, its `pub use`s of its own items by other names (ADR 0596), and what a module exports an item of its own as, which every importer imports it by | `default_exports.rs` | `default_exports`, `default_declared`, `own_default`, `own_aliases`, `alias_named`, `exported_as` |
| Trait dictionaries and evidence | `traits/routes.rs`, `traits/dictionaries.rs`, `traits/impls.rs` | `dictionary`, `evidence_for`, `has_evidence`, `impl_call` |
| Generic arguments and copied defaults | `traits/given.rs` | `in_impl_terms`, `given_evidence`, `given_type_fact`, `resolve_self_instance`, `enter_default` |
| Ambiguous Option payload boxing | `std_types/option.rs` | `some`, `some_value`, `some_literal`, `is_some_box` |
| Whether an Option's payload may be falsy in JS, and so whether it's there, by its truth or against `null` (ADR 0298), and what `#[rust_js::types]` says may be | `std_types/option.rs`; `declarations.rs` | `never_falsy`, `present`, `absent`; `written_falsy` |
| Copy policy | `copies.rs` | `copy_if_needed`, `contains_mutated` |
| Which `Rc`s are counted, by the type each points at and what may be it (ADR 0852) | `analysis/rc_counts.rs` | `counted_rcs`, `counts`, `may_be`, `head` |
| Map/set operations and discarded results | `std_types/map.rs` | `map_values`, `discarded_calls` |
| Cells and plain module statics | `std_types/cell.rs` | `cell_call`, `plain_statics` |
| A field that's the property its cell holds, a `Cell`'s, a `RefCell`'s never counted, a `OnceCell`'s | `representation.rs` | `property_cell`, `held`, `holding` |
| Which `RefCell`s are the objects they hold, a field's plain one, which are never checked, and what a library's contract says of them (ADRs 0362, 0863) | `drops/ref_cells.rs` | `ref_cells`, `is_value_cell`, `plain_ref_cell`, `quiet_ref_cell`, `holds_itself` |
| What a call, std's code's too, and a cell's own impls may borrow (ADRs 0580, 0863) | `analysis/cell_borrows.rs` | `call_asks`, `call_runs_unseen`, `impls_ask`, `may_be_cell` |
| A type equal only to itself, a JS key by itself (ADR 0864) | `std_impls.rs` | `equal_only_to_itself` |
| Std call classification | `recognition/calls.rs` | `classify`, and its answers through `shortcuts.rs` |
| Std identities and layout | `recognition/std_items.rs`, `recognition/types.rs`, `recognition/fmt.rs`, `recognition/iterators.rs` | `StdItem`, `is_std_type`, `is_std_def`, `std_item`, `trait_method`, `cell_value_field`, `deref_target`, `debug_builder`, `builder_step`, `iterator_method` |
| Std coverage catalog | `recognition/registry.rs` | `measured` |
| Compiled std bodies, types and module placement | `recognition/std_items.rs`, `recognition/iterators.rs` | `compiled_std`, `is_compiled_std`, `is_compiled_std_type`, `is_compiled_std_iterator`, `std_home_module`, `memory_intrinsic`, `addresses_memory` |
| Memory-addressing operations | `pointers.rs` | `raw_pointee`, `mir_addresses_memory` |
| Std-private iterator specialization | `recognition/iterators.rs`, `recognition/std_items.rs` | `fused_specialization`, `std_private` |
| Compiled iterator stepping | `iterators.rs` | `std_iterator_steps`, `compiled_step` |
| Generic IntoIterator representation | `iterators.rs` | `into_iter_held_as_it_is`, `into_iter_as_it_is` |
| Function-value mutable arguments | `mut_refs.rs` | `unboxed_by_function` |
| Body module placement | `lower.rs` | `body_module` |
| A parameter's JS name, its body's and its declaration's | `lower.rs` | `parameter_name` |
| Std body capture | `analysis.rs` | `collect_std_bodies` |
| Dictionary cache names | `names.rs` | `dictionary_cache` |
| String offsets in JS units | `mir/offsets.rs`; `std_types/text.rs` | `in_units`, `call_in_units`, `step`, `cut`, `seed`; `found`, `sliced_in_units` |
| A shadow that is the binding it shadows | `mir/tidy/bindings.rs` | `shadowed` |
| A `String` appended to twice | `std_types/text.rs` | `appended` |
| Runtime helper identity and properties | `runtime.rs` | `exports`, `helper_of`, `is_pure`, `names_a_place` |
| A function's rest parameter and its calls' rest arguments | `bindings.rs`; `calls.rs` | `marked_rest`, `takes_rest`; `rest_given`, `spread_rest` |
| The order a struct literal's object lists its fields, and reads them, in | `aggregates.rs` | `written_order` |
| A variant without fields as a body writes it, a numbered enum's its module's `const` (ADR 0722), and a constant rustc computed there | `representation.rs` | `unit_variant`, `const_value` |
| Whether a body reads a `const` or a numbered enum's variant by its name, or, where its module loads, another module's as its value (ADR 0722) | `items.rs` | `reads_by_name` |
| Whether a struct literal's object is across lines, as the literal is written | `aggregates.rs` | `object_layout`, `const_object` |
| An untagged enum's `undefined` and `null`, and what's there of `classify(Some(v))` | `untagged.rs`; `body_queries.rs` | `unit_value`, `holds_nullish`, `told_by_kind`; `given_present`, `present_calls` |
| What an untagged enum's variants are together, a test's and a `switch`'s case | `untagged.rs` | `untagged_test`, `untagged_cases` |
| What `#[rust_js::types]` says a type is to TypeScript, read, or the error saying what it reads | `declarations/written.rs` | `written_types`, `Written`, `validate_types` |
| A string's count of UTF-16 units, its `length` | `recognition/types.rs` | `counts_utf16_units` |
| Whether a type is a JS class, and what each item of its impl is of it | `classes.rs` | `is_class`, `member`, `constructor_of` |
| What a place stores of what's written, a nullable field's `None` `null`, whether a JS value may be `undefined`, and which places store so, however reached | `js/make.rs`; `representation.rs` | `Storage`, `written`, `may_be_undefined`; `nullable_cell_field`, `nullable_cell_handle` |
| What JS is given of a `()`: `undefined`, or a list's `[]` | `representation.rs` | `unit_given`, `UnitTo` |
| A guard `if let Some(v) = value` that holds where its arm of `match js::classify(value)` matched | `body_queries.rs` | `held_guards` |
| An index or a slice's range in bounds, the test a `bool` variable holds, and a `OnceCell` still empty where it's set | `body_queries/ranges.rs`; `mir/bounds.rs` | `judge_index`, `judge_range`, `held_test`, `made_empty`; `indexes_in_bounds` |
| A binding's read of a property, which a getter may be, and a chain's test of one | `binding_calls.rs`; `mir/tidy/logicals.rs` | `answered_by_a_getter`, `Known`; `bound_into` |

**Calls.** `mir/dispatch.rs::mir_call` handles special calls and sends evaluated
arguments to `mir_call_values`. Calls then go to iterator, trait, binding or
crate-function lowering. Recognized std calls go through `mir_std_call` in
the same dispatcher, which asks each kind in `mir/std_calls.rs` of what it
reads alone and runs the lowering of the one that takes the call
(ADR 0791), delegating place-sensitive behavior to `mir/std_calls.rs`
and value-only behavior to `std_types/values.rs` and the owning type.
`std_values` exhaustively matches `Std`. New behavior belongs in its domain
module, not in a growing dispatcher;
the architecture test holds dispatcher sizes.

## Owned output and source origins

`src/js.rs` defines a compiler-owned AST independent of rustc and oxc.
`src/program.rs` carries modules, symbolic imports, declarations and a source
arena. `src/lower/sources.rs` converts source information to owned text and
spans, retaining origins across files and copied bodies. JSX expansion and
formatting preserve or remap those origins.

Linking resolves symbolic references to ordinary names. During planning,
`js::Module::unprintable` rejects unresolved symbols, function holes, drop
arguments, handles and unresolved dynamic imports before printing. Representation
constants such as `js::HELD` and `js::DICTIONARY` belong to the AST; consumers
do not repeat their spelling. So does an enum's shape, `js::TAG` and
`js::positional(i)`, and a `Result`'s, made, tested and read by `js::result`,
which lowering and `src/prepare.rs` both ask ([ADR 0688](decisions/0688-a-results-js-has-one-owner.md)).

`src/to_oxc.rs` and `src/to_oxc/` convert JS to oxc and generate source maps;
`src/format.rs` lays it out with oxfmt and remaps changed text. Declarations
flow from `src/lower/declarations.rs` through `src/typescript.rs` to `typescript/`.
`src/output.rs` coordinates crate transforms, optional application Prettier,
map adjustment and final artifact assembly.

## Libraries and runtime

A compiled library has JS plus a versioned `src/library.rs` contract embedded
in the host manifest. It carries exported item identities, trait requirements,
held bindings, input fingerprints and crate identity; rustc metadata alone
cannot describe JS linkage. Consumers validate library/compiler identity and
required dependencies. Cargo provides build configuration and caching;
`src/cargo.rs` makes compiler identity visible to Cargo's fingerprints.

`@rust-js/std` contains selected std bodies compiled from the pinned toolchain's
MIR. The catalog in `src/lower/recognition/std_items.rs` owns which ones. `bun run std`
regenerates the committed JS and manifest in `std/`; other compilations use
that built-in manifest and import the package. Hand-modeled std operations
remain in their domain modules until migrated.

`src/runtime.rs` owns helper names, dependencies and implementations in
`src/runtime/`; `bun run runtime` generates `runtime/index.js`. Lowering requests
helpers through `Expr::helper`. Linking collects helper dependencies, and
artifact planning selects imports from the prepared tree's actual reads.
The printer receives those imports; it does not infer them from text.

## Publication

`src/manifest.rs` is the compiler/host contract; `tooling/manifest.js` validates
it and remaps virtual paths as structured fields. It reports modules, imports,
source destinations, artifacts, fingerprints and compiler identity, so hosts
do not rediscover them by parsing generated JS or guessing filenames.

`src/output.rs` prepares all artifacts and decides which old outputs are stale.
`src/publish.rs` stages bytes, preserves unchanged files, backs up replacements
and removals, and publishes the manifest last. Ordinary I/O failures trigger
rollback; this is not crash-atomic publication. Cleanup requires the old file
to retain its recorded fingerprint and protects current inputs and outputs.

For disk publication of WASI results, the compiler reports stale candidates in
the manifest. `tooling/publish.js` validates artifacts, checks disk aliases and
fingerprints, and commits them. The in-browser host instead exposes successful
results from its fresh virtual filesystem.

Cargo's copies beside source are a separate publication step:
`tooling/cargo.js::writeInSource` uses compiler-reported destinations and link
ranges, relinks moved imports, and tracks ownership in a ledger. Its stale
copies come from that ledger; `tooling/publish.js::commit` writes them. It does
not independently decide the compiler's output paths.

## Where a change goes

1. Find the semantic owner above. Extend its API or facts rather than adding
   a competing rule. For a new responsibility, update this map and its checks.
2. For std support, recognition identifies the operation; MIR call lowering
   handles places and effects; the type's module handles values. Extend the
   compiled-std catalog when compiling std's body instead of modeling its API.
3. Preserve phase contracts. Rust facts belong upstream; owned JS rewrites
   belong in tidy or preparation according to the facts they require; spelling
   and layout belong in printing; paths and cleanup policy belong in planning.
4. Verify with the relevant native-vs-JS corpus cases, output snapshots,
   boundary matrices and mutations. Architecture tests protect dependencies;
   `test/host-boundaries.test.ts` checks host contracts; integration tests cover
   app builds and native/browser parity.
5. Update the guide for observable changes and record new decisions in an ADR.
   Follow [CONTRIBUTING.md](../CONTRIBUTING.md) and the local/CI limits in
   [AGENTS.md](../AGENTS.md).

## Not yet

[ROADMAP.md](../ROADMAP.md) owns milestones and release qualification;
[the limitations](guide/limitations.md) describe unsupported program behavior.
Do not read current mechanisms as promises of:

- A complete host/browser compatibility matrix or numbered direct-execution
  JS baseline.
- Arbitrary Cargo dependency compatibility or browser Cargo/build-script support.
- Measured application-scale performance budgets.
- Compilation of every std body; the catalog still selects only part of std.

Dispatcher and file-size caps are held in the architecture tests. Use those
checks and reports for current measurements rather than copying counts here.
