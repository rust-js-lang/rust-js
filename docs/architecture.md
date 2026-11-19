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
  compiler and rebuilding matching native dependencies and WASI artifacts,
  and moving `Cargo.toml`'s clippy crates to the rust-clippy commit whose
  tree is the release's `src/tools/clippy` ([ADR 0888](decisions/0888-rust-js-clippy.md)),
  not its `rust-<version>` tag.
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
| Compiler entry, command parsing, Cargo wrapper, panic reporting, the `core` and `alloc` std's bodies are lowered of, built to unwind | `src/main.rs`, `src/command.rs`, `src/cargo.rs`, `src/std_sysroot.rs` |
| `rust-js clippy`'s lints, clippy's crates registered as rustc's (ADR 0888) | `src/clippy.rs` |
| JSX syntax expansion and provenance; which components' props a glob or a prelude names are built by their companion, rustc's names verifying the syntax pass's prediction, and a replayed compile's held diagnostics (ADR 0213, amended) | `src/jsx_syntax.rs`, `src/jsx_syntax/`, `jsx_syntax/resolved.rs` |
| Rust analysis and lowering | `src/lower.rs`, `src/lower/` (detailed below) |
| Owned JS tree, source arena and comments | `src/js.rs`, `src/js/`, `src/program.rs`, `src/comments.rs` |
| Reachability, linking and names | `src/reachability.rs`, `src/link.rs`, `src/names.rs` |
| The compiler's maps and sets, each walked in the order its entries were put in, never a hasher's (ADR 1098); a session's answers that never change, each kept as first found, `OnceMap` (ADR 1100) | `src/collections.rs` |
| Readability and printing | `src/prepare.rs`, `src/prepare/`, `src/to_oxc.rs`, `src/to_oxc/`, `src/format.rs`, `src/format/jsx_shape.rs` (each JSX element's children as React is given them, which a layout keeps, #1807), `vendor/oxc_formatter` (oxc's formatter, with JSX text's edge space beside a child never where a line breaks, #1807), `src/typescript.rs`, `src/prettier.rs`, `src/node_session.rs` (the Node session each of the last two runs, its JSON read whole as UTF-8, ADR 1079); `typescript/` owns the TypeScript declaration model, reading/printing and engine-classified standard-library selection (#1903) |
| Library contracts, configuration, hooks and path utilities; a crate's npm package, what its entry re-exports (ADR 1008) and the subpaths its `exports` resolves (ADR 0770) | `src/library.rs`, `src/library/dependencies.rs` (the libraries a crate is compiled against, read and checked before it, and `@rust-js/std`'s), `src/settings.rs`, `src/hooks.rs`, `src/paths.rs`, `src/packages.rs` |
| Artifact planning, manifest and publication; the program's own files lowering reads, `LocalJs` | `src/output.rs`, `src/output/`, `src/manifest.rs`, `src/publish.rs` |
| The class names a CSS Module exports, the one thing read of a stylesheet, by Lightning CSS (ADR 1133) | `src/css_modules.rs` |
| Runtime helper catalog and implementations | `src/runtime.rs`, `src/runtime/`; generated package in `runtime/` |
| Test child outcomes, own-kill provenance, queued-exit drain and observed-overrun diagnostics | `test/child.ts`; `scripts/checkFast.ts` labels the owner's diagnostic with its job |
| Source-owner controls, workspace lifetime and actual-load proof; filesystem copying, aliases and disk-floor checks | `scripts/source-mutations.ts`; `scripts/source-workspace.ts` is the one filesystem owner, executed on a worker under the existing runner slots |
| Compiled standard-library bodies | `std/lib.rs`, generated `std/*.js` and `std/manifest.json` |
| Native app build and editor check; the command each JS test runner runs a crate's tests by (ADR 1076) | `tooling/cli.js`, `tooling/build.js`, `tooling/cargo.js`, `tooling/check.js`, `tooling/runners.js` |
| Host manifest validation and file publication | `tooling/manifest.js`, `tooling/publish.js` |
| npm/Cargo integration and shipped binding catalog; rust-js's own wasm-bindgen, js-sys and web-sys for an app's dependencies, its catalog `tooling/compat/`'s folders, patched while Cargo's graph has one and they serve each crate that uses one (`settleCompat`, `tooling/compat/covered.txt`), and refused for an app's own crate or a pin (`compatRefused`, `packaged`); `#[wasm_bindgen]`'s ABI table, `tooling/compat/wasm-bindgen-macro/src/abi.rs` (ADR 1112) | `tooling/patch.js`, `tooling/crates.js`, `tooling/compat/`, `tooling/cargo.js` |
| Shipped JS, web, React, Node, Next.js and React Native bindings | `builtins/`, `webapi/`, `react/`, `node/`, `next/`, `react-native/`; the authoritative list is `tooling/crates.js` |
| The js crate's builtins from TypeScript's library | `builtins/library.ts` reads a root from the engine's ESNext graph and writes it; `builtins/render.ts` owns each binding form the hand specs and it write by; `builtins/policy.ts` what TypeScript doesn't say, each entry citing its test (ADR 1172) |
| What every JS object's binding is, `Defined` and `WeakKey`, which webapi's and the builtins' generators write of each class and `test/object-markers.test.ts` holds every crate's `Classed` type to | `scripts/object-markers.ts` (ADR 1172, amended) |
| External crates used by compiler tests | `serde/` builds crates.io dependencies; it is not a shipped binding crate |
| WASI compiler build | `wasm/Cargo.toml`, `wasm/build.sh`, `wasm/patches/` |
| Playground UI and in-browser compiler host | `wasm/web/rust/`, `wasm/web/compiler-client.js`, `wasm/web/compiler-worker.js`, `wasm/web/frame-report.js` |
| Playground build and served assets | `wasm/web/compile-rust.ts`, `wasm/web/site.ts`, `wasm/web/vite.config.ts` |
| Tests and mutation tooling | `test/`, `scripts/mutations.ts`, `scripts/mutations/`, `scripts/matrix.ts` |
| Which tests a change can affect: the test selector, the one answer `bun run push`'s check and CI's suite run (ADR 1101, amended); the test map it reads of the compiler's code, recorded after each check of `main`; the nightly's audit of its misses | `scripts/select.ts`; `scripts/testMap.ts`, `.github/workflows/test-map.yml`; `scripts/selector-audit.ts` |
| The issues' kind × area count, each cluster and the refactor its kind calls for (ADR 1085), the nightly's comment on #1681 | `scripts/clusters.ts`; the kinds' table, `docs/bug-kinds.md` |
| What CI spent each day and what it wasted, the nightly's comment on #1989; which jobs of a push's check always run, and why | `scripts/ci-spend.ts`; `scripts/ci.ts`'s `alwaysRun` |
| Which commit of a batched check broke each test it failed | `scripts/attribute.ts` |
| Packaging, release and CI | `scripts/package-*.ts`, `scripts/ci.ts`, `.github/workflows/` |

## Hosts

An application compiles Rust through `rust-js build`, once per invocation
([ADR 0412](decisions/0412-rust-js-build-is-its-own-process.md)).
It writes JS/JSX and declarations beside the Rust; `--check` compares generated
source files without replacing them. Vite and Next.js consume those files.
They do not host a rust-js compiler plugin or schedule Rust compilation.
`rust-js clippy` lints the app and writes nothing: the editor's check's
Cargo run (`tooling/cargo.js::lintCheck`), with clippy's lint flags in
`RUST_JS_CLIPPY`, which makes `src/cargo.rs` check each crate of the app's
own with `main.rs`'s `Clippy` ([ADR 0888](decisions/0888-rust-js-clippy.md)).

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

Which adapters each body makes in place (`body_queries/in_place.rs`) is
decided in two stages. The ground, in `lower/pipeline.rs`, after the crate's
ground (`analysis/ground.rs`: its libraries, trait impls, cloned types and
capacity facts) gives its `Recognition`, and before the analysis lends the
bodies out: a collect's hint the program can see decides it too (#1327).
Then, in `analyze_crate`, once `back_stepped` knows what's stepped from its
back and before `type_fact_params` reads it, what's held after all
(`decide_held`, #1681). Between the stages only `back_stepped` reads, the
ground alone (`ground_made_at`), of a `rev()`, which the second stage
may take out after it, as nothing it flips is given what the ground made
(`held_links`); every other reader asks `made_at`, which refuses a body
whose stages aren't both decided (`test/architecture-iterators.test.ts`).
Each stage holds THIR calls; a reader asks of MIR's call at a span, of
each THIR call there (`each_call_at`), the one join, so of two a macro's
repetition writes at one span where one is held, both are (#1791, ADR
1151).

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
lower/pipeline.rs: crate ground -> in-place adapters decided -> analyze crate
    |              -> lower bodies -> assemble retained modules
    |                 MIR -> JS AST -> body tidy
    v
program::Unlinked             owned data: no rustc types beyond this boundary
    |
    v
link.rs -> link::Linked       resolve symbols: each import a Name, its reads; collect helpers; reads no words
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
| Driver | `main.rs`, `command.rs`, `cargo.rs`, `clippy.rs`, `std_sysroot.rs` | Every layer |
| Front end | `lower.rs`, `lower/`, `jsx_syntax.rs`, `jsx_syntax/` | Front end and earlier-use owned data/utilities; no linking, preparation or publication |
| Owned output | `js.rs`, `js/`, `program.rs`, `comments.rs`, `link.rs`, `reachability.rs`, `names.rs`, `prepare.rs`, `prepare/`, `output.rs`, `output/`, `publish.rs`, `manifest.rs`, `library.rs`, `runtime.rs`, `settings.rs`, `hooks.rs`, `paths.rs`, `packages.rs`, `collections.rs`, `decisions.rs`, `css_modules.rs` | Owned output; no rustc |
| Printing | `to_oxc.rs`, `to_oxc/`, `format.rs`, `format/`, `typescript.rs`, `prettier.rs`, `node_session.rs` | Printing and owned output; no rustc |

The named orchestration exceptions are `output.rs`, which invokes the printing
adapters, `output/local_js.rs`, which asks `to_oxc.rs` what a JS file of the
program's own loads, and `hooks.rs`, which invokes `format.rs`. Only printing adapters use
oxc. `output.rs` also writes manifest JSON and each test file's registrations
of its tests, by the runner's `test` (ADR 1076).
The architecture test checks top-level dependency cycles; it treats the parts
of `lower/` as one module, not as individually cycle-checked units.

## Inside the front end

Paths in this section are relative to `src/lower/` unless stated otherwise.

State has three scopes: crate facts shared across functions; `FnCx` for a
function's context; and `mir::State` for one body's temporaries, labels,
captures and drop flags. What a body's lowering knows of it before it writes
any of it, its graph, patterns, `Option` writes, offsets and jumps, is its
`LocalFacts` (`mir/local_facts.rs`, ADR 0872), made once before emission and
read-only by type: `State` keeps what emission changes. `bodies.rs` manages
nested-body entry and restoration.
Concern-specific state (drops, trait evidence, formatting, cloning and type
walks) is accessed only by the owners allowed in the architecture tests.

Recognition and query modules answer questions without emitting code.
Recognition owns std names, diagnostic identities and library-layout
assumptions; rustc lang items remain available through rustc's identity API.
`shortcuts.rs` only forwards recognition queries.

| Responsibility | Modules |
| --- | --- |
| Crate facts and validation | `analysis.rs`, `analysis/`, `crate_facts.rs` (what the pipeline found of the crate before any body, which lowering shares, `CrateFacts`; and what the bodies lowered before settled, `Settled`, the pipeline's to write, in the bounded rounds of `pipeline/settle.rs`; one memo, `Lazy`, of the crate's facts alone, ADR 1148), `analysis/type_facts/carried.rs` (what a trait's dictionaries carry of associated types, #476, #990) |
| Body facts | `body_queries.rs`, `body_queries/` (a `let`-`else`'s `Some` of `v.get(i)`, `present_item.rs`) |
| std recognition and coverage | `recognition.rs`, `recognition/`, `shortcuts.rs`, `std_coverage.rs` |
| Function contexts, assembly and source capture, and which module may load before another | `bodies.rs`, `pipeline.rs`, `pipeline/`, `reexports.rs`, `sources.rs`, `library.rs`, `library/bound.rs`, `library/internal.rs` (the npm packages a crate's JS imports through bindings, its libraries' too, and two releases of one binding crate over one package refused, #1799), `library/built_types.rs` (a type built of a function's slots as a library's manifest says it, #737), `library/cells.rs` (what a library's contract says of its fields and its `RefCell`s, the quiet ones a consumer may lend, ADRs 0863, 0906, #1032), `library/changed.rs` (what a library's contract says its code changes in place, which its JS copies and a crate sharing a value with it copies for too, ADRs 0914, 0100, #1871), `library/compiled_here.rs` (what the crate compiles of its libraries itself, and why, ADR 0914), `library/exports.rs` (what a library's manifest says of it, the items its JS exports and each fact its consumers read, ADR 0100), `library/reached.rs` (what the libraries can hand the crate, of what its bodies reach of theirs, of THIR and of MIR alone, ADR 0626, #1217), `library/written_into.rs` (what a library's contract says of its writes into a writer a piece at a time, its types whose dictionaries carry `fmtInto` and the parameters its functions write so, ADR 1135, #1652), `load_order.rs` |
| Which of a library's functions and impls the crate compiles itself, from the MIR its metadata keeps, and where (ADR 0914, amended) | `instances.rs`, `instances/needed.rs` (which of its libraries' functions the crate's calls need it to compile itself, and the refusal of an instance's read of its library's private state), `instances/failing.rs` (whether a call gives a library's function what may fail that it writes or formats through, where its export never fails, #1392) |
| The lowering session's own state: what it finds once of rustc's ids, and its run's flags, kept for exactly the session, `caches(tcx)` checked against it (ADR 1111) | `session.rs` |
| MIR lowering, control flow, calls, places, temporaries and drops | `mir.rs`, `mir/`, `mir/iter/`; where a body's MIR reads and writes a local or a place, `mir/queries/reads.rs`, of `mir/queries/`; an aggregate it makes, `mir/rvalues/aggregate.rs`, of `mir/rvalues/`; an iterator's call `mir_iter_call` leaves, `mir/std_calls/iterators.rs`; a std type's `clone_from`, `mir/std_calls/clone_from.rs` |
| Body readability passes | `mir/tidy.rs`, `mir/tidy/`, `mir/tidy/bindings/`, `mir/tidy/decisions/`, `mir/tidy/jumps/`, `mir/tidy/logicals/`, `mir/tidy/switches/`, `mir/tidy/values/` (a handle made on a place, read and written in place, `values/handles.rs`) |
| Value shape, copy policy, support and diagnostics | `representation.rs`, `representation/`, `copies.rs` (the one constructor of a write in place, `copies/in_place.rs`; what a function React gives again what it kept copies, `copies/kept.rs`; what a library stores handed out as a copy of its own, `copies/stored.rs`, #1907), `support.rs`, `refusal.rs` |
| Values, patterns, calls and places, and an arrow that only passes on its call (ADR 0605) | `aggregates.rs`, `literals.rs`, `patterns.rs`, `calls.rs`, `loops.rs`, `places.rs`, `passed_on.rs` |
| Mutable references, pointers, plain-data layout and unions | `mut_refs.rs`, `pointers.rs` (its catalog of std's pointer functions, `pointers/catalog.rs`; a cast of or to a raw pointer, `pointers/casts.rs`; pointers into what a call lends that's a copy, text's bytes, and a part of a slice made inline, `slice_part`, `pointers/into_copies.rs`; the writes over a place a pointer is taken to or into, `pointers/rewritten.rs`; a `&mut` a function returns that is its parameter, `pointers/returned.rs`; whether a call may keep a raw pointer to what a `&mut` it's given refers to, `pointers/keeps.rs`), `plain.rs`, `unions.rs` |
| Destructors and their facts | `drops.rs`, `drops/`; `mir/drops.rs` follows rustc's drop elaboration; `mir/elaborated.rs` reads rustc's own elaborated body, the one place that does, and compares each `Drop` with rust-js's, `--drops-shadow` (#2112's change D); `drops/glue.rs` lowers a struct's, an enum's and a tuple's drop glue from rustc's own `drop_glue` shim, the one call of `build_drop_shim` (#887) |
| std types and calls | `std_types/`, `std_types/number/`; `std_types/values.rs` dispatches to each type's module |
| Iterators and combinators | `iterators.rs`, `iterators/`, `combinators.rs`, `combinators/` |
| Item references, traits, dictionaries and standard impls; a recursive type's `==`, clone and order, one function for each module; the one way rustc's trait selection is asked, `traits::select`, a binder's regions erased (#1224) | `items.rs`, `traits.rs`, `traits/`, `std_impls.rs`, `std_impls/clones.rs`, `std_impls/defaults.rs`, `std_impls/in_turn.rs`, `std_impls/not_equal.rs`, `ordering.rs`, `recursive_fns.rs` |
| Formatting | `display.rs`, `display/`, `format_args.rs`, `format_spec.rs`; a `format_args!`'s parts as MIR gives them, `mir/std_calls/fmt_parts.rs` |
| Serde and supported external crate APIs | `serde.rs`, `serde/`, `shipped.rs` |
| JSX, JS bindings, untagged values and JS-visible uses | `jsx.rs`, `jsx_api.rs`, `bindings.rs`, `bindings/`, `binding_calls.rs`, `untagged.rs`, `liveness.rs` |
| JS classes, `#[rust_js::class]`, objects of methods, `#[rust_js::object]` (ADR 0943), and module namespaces, `#[rust_js::namespace]` (ADR 1037), whose calls' `self` naming one, and what holds it then, is `classes/namespaces.rs`'; a class that extends a binding's class, an ES class (ADR 1041), `classes/subclasses.rs`'; a class that extends JS's `Error`, `js::ErrorClass`, its `message` and `cause` given `super` and what's refused of it (ADR 1158, #1808), `classes/errors.rs`'; which method is a getter, its own or one of the class it extends, and what's refused of one (ADR 0761, amended; ADR 1041, amended, rd679), `classes/getters.rs`'; a class's form, an ES class or a constructor function, and what's refused of it (ADR 0761, amended, rd682), `classes/forms.rs`'; an ES class's field declarations, `class(es, fields)`, and what's refused of them (ADR 0761, amended, #568), `classes/fields.rs`'; a trait of classes' defaults, each prototype's (ADR 1035), `classes/defaults.rs`'; a trait of classes' method that only gives its field, read in place where it's called, of the crate's HIR or a library's MIR (rd304, #1032), `classes/accessors.rs`'; a class's flattened state, its constructor's keys and each `new`'s (ADR 1044), `classes/states.rs`'; a class's prototype its load replaces, set right after its constructor (ADR 0761, amended, #448), `classes/prototypes.rs`'; what a class's constructor gives, `Self` or an `Rc` of it, and its refusal of one the crate counts (ADR 0761, amended, rd554), and the nullable fields each constructor the crate writes of a struct sets first, present (ADR 0810, amended, rd628), `classes/constructors.rs`' | `classes.rs`; `classes/member_functions.rs`, a member's function made its class's, its `self` `this` and a constructor's literal set on it |
| Exported TypeScript declaration model, and the reader of `#[rust_js::types]` | `declarations.rs`, `declarations/` |

A body's MIR is taken in `Mir::new` (`mir/prepared.rs`, which settles
what can't unwind and what an optimized body left), where
`mir/revealed.rs` (`revealed`)
makes each `impl Trait` in it the type it hides, as rustc's codegen reads
it (#1142): every rule of a body's lowering asks of the type its value is,
and none of `mir/`, `iterators/` or `combinators/` reveals a type itself
(`test/architecture-iterators.test.ts`). `Recognition::reveal` is for
types read elsewhere, a THIR's or a signature's, in a typing environment
that reveals; `eq_sides`, asked in one that may not, of a caller's THIR,
reveals by `mir::revealed` itself (#1229).

MIR determines execution order. `mir/cfg.rs` structures blocks;
`mir/temporaries.rs` folds temporary reads only when intervening effects allow
it. `mir/hints.rs` and `mir/hints/` supply source facts and `mir/offsets.rs` analyzes string
offsets without emission. `mir/tidy/stages.rs` lists the body rewrites, in
stages that each say what they leave a body with; `mir/tidy.rs` runs them and,
where `js::validate::stages_checked()`, checks each stage's
contracts (`mir/tidy/contracts.rs`), its jumps' and declarations' those of
`js/validate.rs`, whose `module` checks each module's tree once more after
preparation, its reads resolved too ([ADR 1143](decisions/1143-every-tree-checked-before-its-printed.md)).
`rust-js --tidy-until <stage/pass>` and `scripts/tidy-trace.ts` show what each
pass does to a crate's JS, and `--tidy-fired <file>` (`mir/tidy/fired.rs`)
and `scripts/tidy-fired.ts`, over the corpus, which passes changed a body;
`--label-census <file>` (`mir/census.rs`) and `scripts/label-census.ts`,
each compiler-made label a body keeps, by the MIR shape that left it
([ADR 0659](decisions/0659-tidy-is-a-list-of-stages-with-contracts.md)).
`mir/tidy/stability.rs` owns whether reading an expression across a call
keeps its value and effects: `plain_field`, `read_past_calls`, and the
recursive fresh-literal predicate. Movement and binding passes consume those
answers rather than repeating the stability rule.
`mir/tidy/moves.rs` decides where a temporary's value may be read in its
place, and why, apart from the rewrite that moves it ([ADR 0689](decisions/0689-a-moves-decision-is-apart-from-the-move.md)).
Its `held_receivers` calls a field's function through its object where the
field is `js::Known::Lasting`, which `mir/places.rs` says of a field of what
a lasting reference is to (`LocalFacts::lasting`, [ADR 1034](decisions/1034-a-fields-function-called-after-statements-keeps-its-object.md)). Type-specific
rewrites remain with their type's owner; downstream `src/prepare.rs` works
only on the completed JS tree, its rewrites each named in
`prepare/passes.rs`'s `PASSES`, which `--prepare-without <pass>` leaves out
and `--prepare-fired <file>` reports; `test/tidy-stages.test.ts` runs every
variant of tidy's stages and preparation's rewrites against native Rust,
and `test/getter-reads.test.ts` compares what each reads of JS-made values
([ADR 1087](decisions/1087-each-readability-rewrite-is-checked-on-and-off.md));
and of the facts those rewrites read, `js::Known`'s, each that only
permits a plainer form is forgotten in turn, `--without-known <fact>`, the
program running the same ([ADR 1127](decisions/1127-a-fact-has-a-role-and-the-js-is-right-without-one-that-permits.md));
and each std function's recogniser is left out in turn,
`--without-recognizer <std path>`, the program refused or running the same
([ADR 1128](decisions/1128-what-no-recogniser-takes-is-refused-or-right.md));
and tidy is told nothing of where the program writes its constructs,
`--no-thir-hints` (`mir/hints.rs`), the program running the same
([ADR 1144](decisions/1144-the-structurer-keeps-what-it-knows-if-measured-so.md), amended);
and nothing MIR's intervals prove is taken, `--without-intervals`
(`analysis/intervals/session.rs`), every check and wrap kept, the program
running the same, a family of variants of its own that a change to the
intervals, the pointers' code or a `&mut`'s runs per push
([ADR 1137](decisions/1137-the-bounds-analysis-on-mir-shadow-first.md), amended, #2112 B). The printer owns spelling and layout, including
expression forms determined while converting to oxc; Rust decisions stay upstream.

### Semantic owners

Each question one module answers, the API the others ask instead of
inspecting its state or JS representation, and, where the answer bottoms
out in one primitive (a rustc query, a trait resolution, a type
predicate, a runtime mark), that primitive. This table is the authority,
and complete for the front end:
test/architecture-owners.test.ts ([ADR 1175](decisions/1175-the-owner-table-kept-complete.md), #2162) holds it
to these rules, and `bun scripts/owners.ts` reports where it stands.

- **Completeness.** Each function of `src/lower.rs` and `src/lower/`
  visible outside its module (`pub`, `pub(crate)`, `pub(in crate::…)`, and
  `pub(super)` in `src/lower.rs` or a module right under `src/lower/`,
  whose parent is the whole front end; not a nested module's `pub(super)`)
  that answers a question, returning `bool`, an `Option` of
  what isn't a JS tree (`Expr`, `ExprKind`, `Stmt`, `StmtKind`, `Pattern`,
  `Prop`, `Function`, `Value`, `js::…`), a fieldless enum or an enum a
  row's API names, or one of those in `R<…>` or a `Result`, is named in a
  row's API cell, by its name or `Type::name`, its file of that row's
  owners; or it's marked `// helper: <reason>` in the comment and
  attribute lines directly above its `fn`. Not both. What's neither is
  listed in test/owners-unclassified.txt, which only falls: a new answer
  needs a row or a mark, and one classified comes off the list.
- **Exclusivity.** A row's primitive is read, outside comments and
  literals, in no source of `src/` (and the runtime's JS) but its owners',
  and in one of theirs.
- **Owners.** An owner is a path from `src/` where it starts so; else a
  module of the front end, `iterators.rs`, which is `src/lower/iterators.rs`
  and its submodules, or a folder, `drops/`; else one at `src/`'s root,
  `names.rs`, `runtime.rs`, or the runtime's, `runtime/steps.js`. Each
  API is defined in the sources. A `|` in a cell is written `\|`.

A row written before #2162 keeps its concept as its question until its
area's sweep phrases it as one.

| Question | Owner | API | Primitive |
|---|---|---|---|
| MIR variable facts | `mir.rs` | `alike_local`, `plain_local`, `note_tested`, `note_falsy` | |
| A name's case as JS writes it: a Rust name camelCase, a thread-local's `SCREAMING_SNAKE` one too, a hyphenated key's variable, a module's namespace (ADRs 0038, 0573) | `casing.rs` | `camel_case`, `variable_case`, `unhyphenated`, `namespace_name`, `upper_first`, `lower_first` | |
| A temporary's name: what made it suggests, rust-analyzer's way, the call the source writes by its name or the program's type it holds, else its role's word, which preparation spells, the first its scope lets it be (`prepare::temporaries`); the words any name lowering picks is made of, a `Role`'s, and what `spill` and `fresh` take, a `Base` (ADR 0569, #890) | `temporary_names.rs` | `call_suggestion`, `held_type_name`, `suggested`, `Role`, `Base` | |
| Does dropping a value of a type run anything, and how is it dropped: its destructor's runs of the crate's types, with the `Rc`s counted, asked before any body is lowered too, given the drops a function's slots take (ADR 0320, #737); may it panic (ADR 0902); does making it a `dyn` lose its drop; is an associated type no type in a body's bounds, given nothing there (#798, #828, #1576); is a value that drops only what its type parameters do taken as one dropping nothing, and is a function given one with a destructor where it takes none, refused at the call (ADR 0190); which functions take only the drops they use (ADR 0300)? | `drops.rs`, `drops/` | `drop_value`, `item_drop`, `drop_function`, `enter_body_drops`, `drop_runs`, `runs`, `drops_given`, `has_drops`, `drop_may_panic`, `dyn_drop_lost`, `no_type_here`, `require_no_drops`, `check_no_drops`, `TakesUsed::contains` | |
| A function's drop slots: their numbering, its type parameters, then its associated types, then the types built of them; a body given its drops keyed by each slot's type (ADR 0421, amended, #887); is a type's drop written with the drops a function is given (ADR 0485), and does generic code have a drop of a type only bounds know (#978)? | `traits/drop_params.rs`; `drops/scopes.rs` | `drop_slot`, `slot_index`, `item_slot`, `slot_item`; `give_drop_param`, `given_drop`, `lend_drop_param`, `holds_given_drops`, `has_generic_drop` | |
| Is a value dropped by closing it, `it.return()`: an iterator closed to drop what it holds, or an adapter holding a function with a destructor (ADRs 0422, 0485, #206); does a closure carry its drop, which `$dispose(f)` runs once (ADRs 0484, 0521, 0522, #710)? | `drops/parts.rs` | `dropped_by_closing`, `carries_its_drop` | |
| Is a lowered body one the drops wrote: a `catch_unwind`'s, `$catchUnwind(f)` (`catches_unwind`), an aborting `finally`'s `try` (`is_aborting`) guarded by `$live()` (`live_guarded`), a cleanup as `cleanup` leaves it (`is_cleanup`); may a `finally` hold a drop, which can't throw and asks nothing of whether one unwinds (#847, #1438); do drops run nothing a program sees (`Known::Unseen`, #1193), and may a drop's statements throw (`may_throw`, one of four answers, #2164)? | `drops/parts.rs` | `catches_unwind`, `is_aborting`, `live_guarded`, `is_cleanup`, `finally_holds`, `unseen`, `may_throw` | |
| JS iterator versus array representation; which of JS's own iterator forms a value is, `values()`, `keys()`, `entries()` or a lazy range (`own_form`, the one answer) | `iterators.rs` | `is_js_iterator`, `js_iterator`, `from_iterator`, `stepped`, `stepped_by`, `stepped_range`, `own_form`, `OwnForm` | |
| Generic iterator evidence | `iterators.rs` | `given_as_it_is`, `iterator_dictionary`, `iterator_evidence`; which functions step one, `analysis/iterator_steps.rs` | |
| Array stepping and lent iterators | `iterators.rs` | `stepped_items`, `is_stepped_items`, `iterated`, `lent_iterator`, `unlent` | |
| Does an iterator know its back, `$nextBack`, and what made it so: its type (`knows_its_back`), what its form holds (`knows_its_back_of`, `knows_its_back_by_form`, `was_made_knowing_its_back`), its maker (`made_knowing_its_back_by_its_maker`, `reversed_made_knowing`), or the crate's stepping it from its back (`made_stepped_from_its_back`, `made_counted_from_its_back`)? And is a string's `Chars` asked whole? (ADRs 0429, 0542, 0545, 0987; #814, #851, #1186) | `iterators.rs` | `knows_its_back`, `knows_its_back_by_form`, `was_made_knowing_its_back`, `form_lacks_a_back`, `made_knowing_its_back_by_its_maker`, `reversed_made_knowing`, `made_stepped_from_its_back`, `made_counted_from_its_back`, `chars_asked_whole`, `reversal`, `reversal_of`, `is_reversed` | |
| Does an iterator know its length, its `len()`: by its type, as an adapter that keeps its source's, or its own runtime form's? (ADR 0456) | `iterators.rs` | `knows_its_length`, `knows_its_length_of`, `len_of_its_own`, `maps_what_knows_its_length`, `knows_its_given_length`, `was_made_knowing_its_length`, `form_lacks_a_length`, `count_bound` | |
| Is an iterator's `size_hint()` std's: known as it's made, held by a compiled std struct, made a `dyn`, or made knowing it where generic code asks it? (#1236, #1333, #1427, #1476) | `iterators.rs` | `held_hint_known`, `hint_known_as_made`, `hint_known_made_dyn`, `struct_hint_known`, `made_knowing_its_hint`, `form_knows_no_hint`, `binding_made` | |
| Is a type parameter's iterator given as it is, or given by its trait, and what bounds it as an iterator? (ADRs 0411, 0488, 0511, 0551, 0552) | `iterators.rs` | `iterator_given_as_it_is`, `param_given_as_it_is`, `holds_into_iter`, `given_by_its_trait`, `bound_as_iterator`, `bound_by_iterator`, `named_by_another_bound` | |
| What form is a lowered iterator value, `IterValue`: a JS value, closed, lent, enumerated, zipped, a stepped range, a string's, an array made, a map of items, mapped lazily; and which bounds of it are read? | `iterators.rs` | `IterValue::of`, `IterValue::js`, `closed`, `lent`, `lent_of`, `is_lent`, `is_knowing`, `is_range_stepped`, `enumerated_of`, `zip_parts`, `made_string_iterator`, `bounds_read`, `made_array`, `is_items_map`, `lazily_mapped` | |
| What does a `Vec` grow by, and does an `extend` or a `collect` reserve its room by a hint the program asks? (#1236, #1333) | `iterators.rs` | `collect_hint_seen`, `extend_grows`, `extend_reserves`, `extend_room`, `given_growth_kept`, `grows_vec_by`, `room_of`, `vec_grown_by` | |
| Does the crate's own iterator step itself: its own `next_back`, its `DoubleEndedIterator` and `ExactSizeIterator`; is an iterator cloned apart; what does an `IntoIterator` make, and is it a slot iterated? (ADRs 0055, 0561) | `iterators.rs` | `own_next_back`, `user_double_ended`, `user_exact_size`, `cloned_apart`, `iterator_cloned_apart`, `made_into_iter_of`, `iterated_slot` | |
| How does an eager or in-place iterator call read: are its items compared never NaN, does it fold as it steps, sort as JS strings, import `fuse`; does an in-place step do anything, and may a JS expression it makes throw (`may_throw`, asked crate-wide; see the filed issue)? | `iterators.rs` | `compares_never_nan`, `folds_as_it_steps`, `sorts_as_js_strings`, `fuse_import`, `does`, `may_throw` | |
| How is a MIR iterator call lowered: what a consumer consumes and where, what `collect` and `extend` fold or collect in place, what a source steps (an array, a counted chain), whether a type implements `Iterator` by its `Item`, what's copied of an iterator, which slice a closing one sees, and whether an impl may unwind? | `mir/iter.rs`; `mir/std_calls/iterators.rs` | `consumed_at`, `folds_held_function`, `slice_seen`, `collects_by_fold`, `collects_in_place`, `extends_by_fold`, `impl_may_unwind`, `try_processed_into`, `consumed_function`, `copied_given`, `is_its_source`, `counted_chain`, `implements_iterator`, `steps_an_array`, `mir_iter_std_call` | |
| Which std iterator type, trait or method is this, by rustc's item or path: an array's, a slice's, a deque's, a tree's, a `dyn`'s, the crate's own, an adapter, double-ended, exact-size, fused, peekable; a search, a split, an extension, an accumulation; what it holds, folds, consumes or drops? | `recognition/iterators.rs` | `accumulation`, `accumulation_of`, `array_source`, `closed_on_drop`, `compares`, `double_ended_iterator`, `drops_passed_as_slice`, `dyn_iter_trait`, `exact_size_iterator`, `extract_if_test`, `extreme`, `flattened_inner`, `flattened_options`, `folds_what_it_consumes`, `from_iter_given`, `generic_shunt`, `held_function`, `held_function_folded`, `holds_its_source`, `implies_iterator`, `index_by_slice_index`, `is_array_iter`, `is_deque_iter`, `is_double_ended`, `is_dyn_iter`, `is_exact_size`, `is_extend`, `is_extension_entry`, `is_fused_iterator`, `is_generic_iter`, `is_given_iterable`, `is_iter_adapter`, `is_iter_empty`, `is_iterator_extension`, `is_iterator_position`, `is_peekable`, `is_reverse`, `is_slice_items`, `is_slice_iter_mut`, `is_str_search`, `is_str_split`, `is_sum_or_product`, `is_tree_iter`, `is_user_iterator`, `is_vec_splice`, `iterates_as_itself`, `iterates_items`, `iterator_item`, `moved`, `own_back_method`, `own_iterator_method`, `own_len`, `reversed`, `rfolds_what_it_consumes`, `search_knows_its_back`, `search_runs_code`, `shows_its_slice`, `std_extend`, `steps_from_its_back`, `steps_its_array`, `steps_past_a_slice`, `tries`, `values_from_iter` | |
| What does an adapter's combinator hold and run: its sources and subject, a subject read twice, an `eq` that's identity, a slice's item, a callback's argument handled, one unrun dropped? | `combinators.rs` | `holds_its_sources`, `subject`, `eq_is_identity`, `slice_item`, `drops_unrun`, `handled_argument`, `subject_read_twice` | |
| What a `&mut` to a type is in JS, the one answer each operation on one matches: a box, a generic code's box, the object, a guard's object, a value given as it is or the value itself (#614, ADR 1086); and what's moved out of one whole, a copy apart, `$apart`, or a flattened state's copy of its variant's keys (#658, #659, ADR 1044, amended) | `representation/mut_ref.rs` | `MutRefForm` and its methods, each a match of every form, `is_cell`, `is_generic_box`, `is_the_object`, `is_the_value`, `written_in_place` (no site names or compares a form, #821), `mut_ref_form`, `mut_ref_form_in`, `param_form`, `param_is_box`, `handles_mut`, `moved_out`, `is_flattened_state` | |
| What generic code needs of its type parameters, one catalogue: each fact, its route, given at run time, one form adapted at the boundary or decided over every instance, and its owner; and the one boundary where a form is adapted, a raw pointer's of a type parameter, a handle, `$ptrTo(x)` where the caller's is the value itself, and a `&mut`'s, generic code's box, `$cellOf(object)` where the caller's is the object, `.value` back, at a call, a function value and where a generic function is a function pointer (ADRs 0454, 1114, 1115) | `generic_facts.rs` | `Form`, `form_differs`, `declares_box`, `is_generic_box_in`, `form_given`, `form_back` | |
| Whether a call may let a reference it's given out as a raw pointer, of its signature alone, and so is given the pointer to the place, not a box of a copy: a function's, a closure's, a function pointer's; the references a body lends so, made as `&raw mut` of their places (ADR 1114) | `mut_refs.rs`; `pointers/lending.rs` | `lets_pointers_out`, `given_its_pointer`, `sig_given_its_pointer`, `given_its_pointer_here`, `signature_of`; `lent_as_pointers` | |
| Whether a crate's own function returns its `&mut` parameter, on every path a reborrow of it through casts that keep its place, so a call of it gives back the pointer it was given (ADR 1114, #1513) | `pointers/returned.rs` | `returns_its_parameter`, `reborrowed_from` | |
| Whether a call may keep a raw pointer to what a `&mut` it's given refers to, past its call: every body it runs walked, the one answer lowering asks of a `&mut` given generic code and the MIR intervals of a `Vec` given the crate's code (#1781, #1947; test/architecture-pointers.test.ts) | `pointers/keeps.rs` | `pointer_may_keep`, `Libraries` | |
| Which writes of a place a raw pointer is taken to or into, a local or a field written whole again, are over what it holds, `$assign(p, v)`, the objects inside it kept, made, or refused after a move (#1484, ADR 0562); the lowering writes them and the crate's analysis of what changes in place reads them | `pointers/rewritten.rs`; the write, `mir/assign/rewritten.rs` | `rewritten_places`, `rewritten`, `Rewritten`, `rewritten_writes`, `rewritten_write` | |
| How a place holds an `Option`'s `None`, the one answer each writer asks: a plain one's `undefined`, a `#[rust_js::nullable]` one's `null`, an optional nullable field's left out, a nested one's `Some(None)` `null`, a held value's, what a binding's call gives as it declares it (ADRs 0196, 0275, 0298, 0586, 0810, 0861, 1017, #819); what a write of it stores, `Storage::NullForNone` or as given; and what a reader may trust of it, of who writes it, `Known::NoneNull` or `Known::NoneUndefined` | `representation/nones.rs` | `NoneForm`, `NonePlace`, `Writers`, `none_form`, `null_params` | |
| Mutable-reference boxes and a number's copy-back; what a call is given for each argument, a `&mut` to a held `&mut` to an object in generic code's form (#1300) | `mut_refs.rs`; `mir/assign/boxed_args.rs` | `boxed`, `copied_back`, `mut_borrowed`, `nested_mut_object`; `boxed_args` | |
| Which `&mut` is the place of what it refers to, a whole write through it the place's, MIR's answer, and which is read from where one's kept, the object's, `$assign` (#466, ADR 0099, amended) | `mir/local_facts.rs` | `place_reference` | |
| Formatter output and MIR writer calls | `display.rs`; `mir/writer.rs` | `formatter_answer`, `written_of`, `builder_written`, `mir_writer_call` | |
| Whether a call's `Result` is only given on by `?` or returned, so its error may be thrown, or looked at, so it's a value: a `write!` into a `Formatter`'s and a serde protocol call's (#1524, #1545) | `mir/result_use.rs` | `result_use`, `ResultUse` | |
| Which type parameters each function shows with a `Formatter`'s options, pretty or in hex, of its placeholders and through its calls, a library's by its manifest's `shown_pretty` and `shown_hex`; and the types the crate gives one, and their fields, whose writers take those options where the crate's own don't (ADR 0137, amended, #1331) | `analysis/shown_params.rs`; `display.rs` | `shown_params`, `ShownParams`, `Shown`; `compiled_writer_takes_options`, `with_shown` | |
| Function and binding references | `items.rs` | `fn_ref`, `js_ref`, `resolve_instance` | |
| Which bindings do modules hold, a renaming private `use` of a global too (ADR 0622, amended), and how does a caller name one: does its module export it by its name (ADR 0773), what does a caller naming it through its module import, the module's default or its name (ADR 0240), and which module does a `use` name it through, imported where the `use` is (ADR 1063, amended; rd609, rd649)? | `bindings/held.rs`; `items.rs` | `held`, `held_path`, `exported_by_name`, `import`, `use_read`; `binding_ref` | |
| Which paths name a function through the module that has it, the crate's or a library's, read through that module's namespace, `Plugin.extractEvents`, and the namespace's name (rd114, ADR 1018) | `analysis/namespaces.rs`; `items.rs` | `module_paths`, `namespace_of`; `named_fn_ref` | |
| What is a module's default export, and is an item marked `#[rust_js::export_default]` the default itself (ADR 0625); what are its `pub use`s of its own items by other names (ADR 0596), and what does a module export an item of its own as, which every importer imports it by? | `default_exports.rs` | `default_exports`, `defaults`, `default_declared`, `held_by_default`, `own_aliases`, `alias_named`, `exported_as` | |
| The name a `use` imports or exports its item by: its `#[rust_js::name]`, renamed or not, else the item's own or its alias camelCased (ADR 0622, amended) | `bindings.rs` | `use_name` | |
| A struct's field's key in JS, its name or the symbol `#[rust_js::name(SYMBOL)]` names it by, read of the use the `jsx!` pass writes beside its struct, and where a symbol can't name one (ADR 1051) | `bindings.rs`; `bindings/symbol_names.rs`; `field_keys.rs`; `jsx_syntax/parser.rs` | `field_key`, `FieldKey`; `symbol_name`, `validate`; `field_member`, `field_symbol`; `field_symbol_uses` | |
| What a module's `pub use`s re-export, from where, by what name, in what order, where each name's leaf is written, whose comments are its (ADR 0874, amended, rd338), and which are refused (ADRs 0240, 0874), and a function re-exported in its place (ADR 1064); where each is among the imports (ADR 1063, amended, rd620); which a module's file has; what another module names through one that re-exports it, imported from there where its load makes no cycle (ADR 0874, amended, rd650); what a module exports an item as, through its `pub use`s and globs (#592), which a library's contract says of its re-exports, for a consumer's import through them (rd704); a glob's, marked so, `export * from` where its names are all its module's file exports, which output planning decides against `output::exports` (ADR 0874, amended, #593); is a `use` of types alone, which `pub use` re-exports an item, what a module exports it as by a path's name (`exported_in`), is a module's only job re-exporting (ADR 1008, amended), does a glob give it a value (#592), and is the text between two items only attributes, which joins a run of `pub use`s (ADR 0874) | `reexports.rs`; `uses.rs` (each name a use declaration imports, its path whole, of 1.100's HIR, whose list of uses is one item, ADR 0109 amended); `analysis.rs`; `load_order.rs`; `library.rs`; `output.rs` | `reexports`, `assembled`, `forwarded_functions`, `forwarded_from`, `place`, `imported_through`, `exported_by`, `reexported_items`, `is_of_types_alone`, `reexporting_use`, `exported_in`, `reexports_alone`, `globs_values`, `only_attributes`; `is_reexport`; `imports_without_cycle`; `import_key`; `every_export` | |
| Trait dictionaries and evidence | `traits/routes.rs`, `traits/dictionaries.rs`, `traits/dictionaries/declared.rs` (what a body declares once at its top of the dictionaries it makes, ADR 0979), `traits/dictionaries/collecting.rs` (a std collection's from_iter as generic code's collect calls it, its dictionary's entry, #1239), `traits/dictionaries/conversions.rs` (a conversion's dictionary, std's Into and From), `traits/dictionaries/references.rs` (a reference's dictionary, std's impl that passes a trait on to what it refers to, through generic code's box of a mutable reference, #1414), `traits/impls.rs`, `traits/impls/defaults.rs` (a trait's default body copied into an impl's dictionary, ADR 0495) | `dictionary`, `evidence_for`, `has_evidence`, `impl_call` | |
| Generic arguments and copied defaults, and the names what a function is given reads (ADR 0979, amended) | `traits/given.rs` | `default_args`, `default_typing_env`, `copied_default_args`, `given_evidence`, `given_type_fact`, `resolve_self_instance`, `enter_default`, `given_names` | |
| Ambiguous Option payload boxing | `std_types/option.rs` | `some`, `some_value`, `some_literal`, `is_some_box` | |
| Whether an `Option` is there, its test against `null` whatever it holds (ADR 0298, amended by the user), and what a test of being set tests; one made where it's tested of a `Some`'s value that's there, `Known::Present`, its test, never of one only never `undefined`, `Known::Defined` (ADR 0910, amended, #539) | `js/make.rs`; `js/stmt_walk.rs` | `presence`, `present`, `defined`; `null_test` | |
| An object's keys of `undefined`, a `None`'s, left out as JS leaves them out: not after a spread, nor one a comment follows on its line where its field isn't optional, nor a cell's, set later (ADR 0280, amended, #538, #575) | `js/undefined_keys.rs` | `left_out` | |
| Copy policy, whether a type is `Copy` at all, as rustc's trait solver answers (`is_copy`, the one place that asks), and of a library, the copies it leaves out, which its contract says, `Library.elided` (ADR 0914, amended; #2112's change D); a tagged enum's variants written once for those of the same fields, a copy's, a clone's and a comparison's (ADR 1160) | `copies.rs` | `copy_if_needed`, `contains_mutated`, `is_copy`, `variant_groups`, `variants_tested` | |
| The functions JS gives again what it kept, read of `#[rust_js::gives_again]`: a component, whose return is React's `Rendered`, and each callback a binding names, a reducer (#1723) | `analysis/given_again.rs` | `given_again` | |
| What a function JS gives again copies of what it was given by value: where it changes it in place itself, at its start; where it changes a part moved out, or gives one to the crate's own code, where it's moved; a copy sharing nothing that may change with JS's (ADR 0020, amended, #1723) | `copies/kept.rs` | `kept_copies`, `kept_copy`, `kept_may_change` | |
| A write in place, `$assign`, `$take`, `$exchange` or a `$cellOf` cell: made only by asking the copy policy whether the written type's copies are left out, refused where they are (#1665, #1672); a compile forgets types of the mutation set, `--without-mutated <type>`, to hold the refusal by a test of its own | `copies/in_place.rs` | `in_place`, `InPlaceWriter` | |
| Which std functions and constants are JS's `Math`'s, for a call, a `fn`, `js::is_defined` and a constant's read (ADR 0887) | `std_types/number/math.rs` | `function`, `math_function`, `constant`, `written_constant` | |
| A closure's handle on itself, the `Rc` of a `RefCell` of the `Weak` its own `Rc` was made of (ADR 0843, amended), or callbacks' on what owns them, the `Rc` of a `OnceCell` of the `Weak` of the `Rc` the call they're given to returns (ADR 0317, amended), or of that `Rc` itself, whoever has them (#577), or callbacks an executor makes on its call's value, the `Rc` of a `Cell` set to it next (rd621): what of their maker and of them is no JS, their own name, the owner's `let` or the value's variable, its `unwrap`s too where its call is quiet (rd621), and which upgrades ask no count | `analysis/self_handles.rs`; `analysis/self_handles/owners.rs`; `analysis/self_handles/value_cells.rs` | `SelfHandles`, `asks_no_count`, `uncounted`, `quiet`, `made_of`, `read_by`, `declared`, `unlowered`, `owned_by_return`; `owner_made`, `direct`; `value_made` | |
| Which of a function's own nullable parameters each of the crate's calls gives `null` or a value, which its body tests `!== null`, as lowering sees each call, a `function::bind` of each parameter one too; which of the crate's functions return `null` or a value at each return, a body that asked before it was found lowered again; a library's by its contract, which its consumers' calls give `?? null` and never use as a value (ADR 0861, amended); which nullable thread-locals nothing reads apart, `null` from `undefined`, whose writes store what they're given as it is (#665); which closures a `with` gives a thread-local of no interior mutability that a test proved `Some` where they're made (#1933) | `analysis/nulled_params.rs`, `analysis/nulled_params/read_alike.rs`, `analysis/nulled_params/present_items.rs`; `library.rs` | `NulledParams`, `scan`, `may_have`, `record`, `is_nulled`, `returns_nulled`, `returned_nulled`, `may_return_nulled`, `bound_all`, `again`, `found`, `reads_alike`, `given_present`; `nulled_param`, `has_nulled_params` | |
| Whether an `Option` is `null` or a value already, which a nullable place stores as it is: what's given or kept in it, each write of a variable so, a shared `as_deref()` of such, and a function's every return (ADRs 0275, 0841, 0861); a nullable thread-local's whole read, whose value `mir/structure.rs` marks `Known::Defined` where it's lowered (ADR 0949, amended, #539); which places store `null` for `None` however they're written, the owner's answer of the MIR place's field or parameter, and a `&mut` to one, a handle that stores so (ADR 1078, #541); what a read of a field, or of a reference to one, may trust its `None` is, the owner's of who writes it (#819); whether a struct's `RefCell` field read again is proved there by a test of the same field, nothing between but that field's own reads, of the program's own data (#1933, #1950); whether a `&mut` lend of an `Option` local never takes its `Some` away, so its writes still say what it is (#1933); whether a test that `undefined` never passes, `js::strict_equals(x, None)` false or `js::classify(x)` not `Kind::Undefined`, proves a read defined there, through a `matches!`'s `bool` too (#1933); whether an arm of `js::classify(x)` that neither `undefined` nor `null` takes proves `x` there (#1933); whether a closure's read of what its `with` lends it is of a thread-local `Some` wherever the closure runs (#1933); whether a thread-local's `Cell` read again, `KEY.get()`, is proved there by a test of another `KEY.get()`, nothing between but its own reads (#1933); whether what a closure's shared capture of an `Option` refers to is proved by a test of it in the closure (#1933); a local written once by a copy of one proved where it's written; and a local proved present on every incoming path, by a test or a later write of `Some`. Never tidy's: no presence rewrite of the printed JS (test/architecture-nones.test.ts) | `mir/nulled.rs`; `mir/nulled/field_reads.rs`; `mir/nulled/lends.rs`; `mir/nulled/defined.rs`; `mir/nulled/items.rs`; `mir/nulled/captured.rs`; `mir/nulled/presence_paths.rs`; `mir/nulled/proofs.rs` | `field_read_proved`, `ways_from_test`, `lend_keeps_some`, `proved_defined`, `classified_present_edge`, `present_item_read`, `capture_read_proved`, `present_on_all_paths`, `place_none_form`, `field_none_trust`, `read_none_trust`, `nullable_ref`, `stored_given`, `nulled`, `given_on_param`, `call_nulled`, `reads_nullable_local`, `alike_reads`, `note_nulled_returns`, `record_bound` | |
| Which of the crate's `Option`s may hold a `None` JS gave, `null`, where Rust's is `undefined`: a flow-insensitive taint of every body's MIR, its locals, struct and capture fields and statics, from what JS or what isn't seen gives (a parameter of what JS or a consumer calls, a binding's result, a nullable place, a closure given where its callers aren't seen, another crate's) through moves, references both ways, the crate's calls and std's by the `Option` types they carry; and the exits, where a `None` leaves for JS as `undefined` alone (a binding's optional argument, what a function JS calls returns, a `pub` struct's plain `Option` field, an argument a JS callback or a JS-made trait object's method is given, what `function::bind` binds), each of which gives `?? undefined` where what it gives may hold one. Returns keep an explicitly written null when the outer option writes are known, even if a binding gives optional fields inside its value (#1840, #1843; ADRs 0030 and 0935, amended) | `analysis/nulled_params/js_nones.rs`; `mir/nulled/one_none.rs` | `JsNones`, `may_hold_null`, `one_none`, `returns_one_none`, `field_takes_one_none`; `none_trusted`, `one_none_given`, `ones_given_to_a_binding`, `ones_given_to_a_callback`, `ones_given_to_a_dyn`, `one_none_returned`, `one_none_field` | |
| Whether a binding's parameter is a native optional argument, TypeScript's `x?`, its `None` left out where it's last: an `Option`, not a setter's value nor a `#[rust_js::nullable]` one; the one owner, asked of `binding_calls`, `dispatch`, `items` and `mir/nulled/one_none.rs` alike (ADRs 0443, 0648; #2112 C S3a) | `bindings.rs` | `is_optional_param` | |
| Which parameters what a function returns alone owns: each moved into a field of the struct its returned `Rc` points at, that nothing reads but to call through it; and which of them are quiet, no body naming such a field among what it may run before it returns (`may_run`); a library's by its contract (ADR 0317, amended) | `analysis/owned_by_return.rs`; `library.rs` | `OwnedByReturn`, `owns`, `of`, `find_quiet`, `quiet`, `quiet_of`; `owned_by_return`, `quiet_by_return` | |
| Which private struct is a function's scope, its one value made by each call of it, held by nothing but its methods and that function's closures, whose methods are then functions declared in it, closing over the value (ADR 1043); each lowered there, a body inside the function's, and its calls, its `self` left out | `analysis/scopes.rs`; `mir/scopes.rs`; `mir/calls.rs` | `Scopes`, `of_function`, `of_method`, `retain`; `declare_scope_functions`; `scope_called` | |
| Which `Rc`s are counted, by the type each points at and what may be it (ADR 0852), an `Rc<dyn Fn>` compared by identity where one may be made of a function made apart (ADR 0959) | `analysis/rc_counts.rs` | `counted_rcs`, `counts`, `may_be`, `head` | |
| How a counted `Rc` is held, read, made and counted: a box, a `dyn`'s pair (ADR 0520), a `#[rust_js::class]`'s instance with its counts hidden beside it, or, of a type parameter a counted class may be, either, told apart as it runs (#1228); its runtime, src/runtime/rc.js | `std_types/rc.rs` | `RcForm`, `rc_form`, `counted_pointee`, `rc_count`, `new_counted_rc`, `counted_as_instance` | |
| Which library functions and impls the crate compiles itself: given more than they take, its lowering asks for what their library's JS hasn't, or their library's contract says they leave out copies of what the crate changes in place (`Library.elided`, #2112's change D), or the impl of a trait's method whose own `where` asks of a caller's parameter what no contract says (`where_unsaid`), or a writer given more options than its library's JS takes (#1026), or one whose JS may merge what the crate compares, listed by its library's contract (#1052), or what may fail that it writes or formats through where its export never fails (#1392); their bodies and modules; which of a library's are the crate's own, which its JS doesn't export that the crate compiles, and which are called through their exported dictionary (ADR 0914, amended) | `instances.rs`, `instances/needed.rs`, `instances/failing.rs`; `library.rs`, `library/compiled_here.rs`, `library/elided.rs`; `traits/impls.rs` | `Instances`, `needed`, `fails_given`, `asked`, `Copying`, `instance_bodies`; `is_instance`, `elided`, `elided_through_calls`, `compiles`, `compiled`, `writes_more`, `merges_identities`, `merged_here`, `merging`, `through_dictionary`, `exported_names`; `dictionary_entry` | |
| How generic code calls what it's given: each closure rustc infers `Fn` or `FnMut` a call by value, `FnOnce`'s `call_once`, may call, refused where it's made unless its body is its `call_once`'s; each type parameter whose value such a call calls, where a boxed `Fn` given is given as one whose call drops it; and a boxed `Fn` called so where it's held in what's given, refused (#2031, #2033, #2050) | `analysis/closures/called.rs`; `instances.rs`; `analysis/closures.rs` | `called`; `on_the_way`, `givings`, `held_of`; `called_by_value`, `param_called_once`, `held_fn_dyn` | |
| What a body of the crate's may run and read of the crate's, the one rule: what it calls, std's calls by what std runs of what it's given (`std_runs`), a library's or a binding's by what it's given, a fn pointer's targets, a trait call's impls, each thread-local's initializer it reads, and the destructors its drops run (`drop_runs`); none where it may run what rust-js can't follow (ADRs 1006, 0317, amended) | `analysis/may_run.rs`; `analysis/cell_borrows.rs` | `Runs`, `may_run`; `std_runs` | |
| Whether an item is rust-js's, the crate's own or a library's, whose JS, dictionaries and contract are rust-js's whichever crate is compiled now: never `id.is_local()` alone, which a library's own compile and its consumer's answer apart (the `is_local()` audit, ADR 0914, amended) | `library.rs` | `compiled_by_rust_js`, `in_library` | |
| Whether a body's local is a variable the program named: none of a body whose locals' kinds rustc cleared, a constant's it evaluated, a `const fn` given a const generic's argument among them, or another crate's, whose `LocalDecl::is_user_variable` panics (#1281) | `mir/locals.rs` | `user_variable` | |
| Where each thread-local is made (ADR 1006, amended): as its module loads unless that would show, what its initializer runs, all `may_run` finds, printing, panicking, reading state what may run after the load writes or writing state it touches (#554), what a `with` lends told apart by its struct's fields (#616), or its module's load can't make it first, one predicate; a private one of those made where it's first used, a read of it its call (ADR 0037, amended), a `pub` one refused | `analysis/lazy_locals.rs`; `pipeline/load_cycles.rs`; `items.rs` | `lazy_locals`, `LazyLocals`; `made_in_cycles`, `out_of_order`; `made_where_used`, `item_read` | |
| Map/set operations and discarded results | `std_types/map.rs` | `map_values`, `discarded_calls` | |
| A map's entry as tidy leaves it: its value put in where it's missing, then used | `std_types/map_entries.rs` | `inserted_entries`, `counted_entries` | |
| Whether a B-tree is std's own nodes, where its keys' order can be seen or isn't total, and what its calls are given, its range and `extract_if` (#926, ADR 0404); the nodes themselves are src/runtime/ord_tree.js, std's `node.rs` and the rest ported | `std_types/btree.rs` | `tree_of_nodes`, `tree_range_values` | |
| A binding's keyed read of what may look like `None`, its `Some(None)` told from a missing key, a `Map`'s `get` by `has` and a `Dict`'s by its own key (ADRs 0225, 0884); of a collection that holds its value's `None` as `null`, `#[rust_js::nullable(V)]` of its type, each write `null` and its read held where it's made JS's read itself (ADR 0884, amended) | `binding_calls.rs`; `bindings.rs`; `bindings/nullable.rs` | `gives_boxed_option`, `held_read`; `reads_a_key`; `nullable_values`, `takes_nullable`, `takes_nullable_entries`, `reads_a_nullable_value`, `reject_nullable_held` | |
| Cells and plain module statics | `std_types/cell.rs`; `std_types/cell/variables.rs` | `cell_call`; `plain_statics`, `nullable_made`, `crate_imported_variables`, `imported_variables_through` | |
| Which of std's byte sources a type is, `&[u8]` or a `Cursor`, the methods its runtime dictionary holds (`$sliceReader`, `$cursorReader`, src/runtime/readers.js), and which of std's io calls a recognizer answers; the sources' and the forwarding impls' dictionaries; what `io::Read`, `BufRead` and `Seek` provide is std's own body, `@rust-js/std`'s `io.js` (#1156, ADR 0425 amended) | `recognition/io.rs`; `std_types/io.rs`; `traits/dictionaries.rs` | `byte_source`, `io_op`, `in_io_dictionary`, `of_io_reading`; `io_call`; `byte_source_dictionary` | |
| Which thread-locals and `static mut`s are their module's variables, exported ones `export let`, what other modules read of them, and which of their writes are refused (ADRs 0270, 0096, amended); which of a library's the crate borrows quietly (ADR 0863, amended) | `analysis/plain_locals.rs`; `std_types/cell/variables.rs` | `plain_thread_locals`, `quiet_library_keys`, `module_statics`, `outside_writes`, `holds_a_value`, `refuse_boxed_export`; `imported_variables` | |
| A field that's the property its cell holds, a `Cell`'s, a struct's `RefCell`'s by its declaration, a `OnceCell`'s; and the one cell of a `RefCell` field lent or checked, `$fieldCell` (ADR 0863, amended) | `representation.rs` | `property_cell`, `held`, `holding`, `uncounted` | |
| Which `RefCell`s are the objects they hold, which struct's fields hold their value by their declaration and which of those no borrow of which can meet another, which are never checked, where one that isn't is first lent or borrowed where another borrow may be, and what a library's contract says of them (ADRs 0362, 0863) | `drops/ref_cells.rs` | `ref_cells`, `is_value_cell`, `holds_value`, `quiet_field`, `quiet_ref_cell`, `quiet_class`, `quiet_cell_borrow`, `holds_itself`, `loud_at` | |
| Whether evaluating an expression can't ask whether a cell is borrowed but by a borrow's own call of it, what `Finder::held_quietly` asks of a guard a `let` holds (ADRs 0328, 0580, 0863) | `drops/facts/quiet.rs` | `Quiet`, `ended` | |
| What a body's patterns bind of its `RefCell` fields: a field's own place, or a cell of its own (ADRs 0362, 0863) | `drops/ref_cells/bound.rs` | `bound_ref_cells`, `bound_place` | |
| The `RefCell` borrows of a body compiled from another crate's MIR, which has no THIR, each counted (ADRs 0863, 0914; #1325) | `drops/ref_cells/instances.rs` | `mir_borrows` | |
| Where the cell a borrow is of comes from, as a body's THIR says: what's lent, a field of a struct or of what a call gives, a thread-local's key, the parts a cell holds, the places a body makes and reads as properties (ADRs 0362, 0863) | `drops/ref_cells/places.rs` | `cell_lent`, `fields_given`, `field_of`, `borrows_its_cell`, `thread_local_key`, `held_parts`, `made_places`, `property_reads` | |
| What a `dyn` of the crate's own trait drops, nothing where no implementor is generic or drops (ADR 0320); a program compiled for its tests says which in its contract, where a consumer's `dyn` of one of what drops is refused; and the trait constants its dictionaries carry, its own generic code's, a consumer's read of another refused (ADR 1076, amended, #597) | `drops/types.rs`, `analysis/programs.rs`, `analysis/generic_consts.rs`; `library.rs` | `own_dyns_drop_nothing`, `generic_consts`; `drops_no_dyn_of`, `lacks_dictionary_const` | |
| Is a `dyn` a pair, `{ value, impl }`, and of which trait: through its pointer (`is_pair_dyn`, `dyn_trait_ref`), a `RefCell<dyn T>`'s or one whose `RefCell` is its pair (ADR 0397), a struct's unsized last field (ADRs 0409, 0436); can a trait's `dyn` be a pair at all (ADRs 0049, 0099, 0141)? | `traits/dyns.rs` | `is_pair_dyn`, `dyn_trait_ref`, `dyn_cell_trait`, `cell_dyn_trait`, `dyn_tail`, `dyn_supported` | |
| What does a `From::from` or `Into::into` call convert, from which type to which (three more answers written in place, #2166)? | `traits/dyns.rs` | `converted` | |
| Which cells each borrow can be of (ADR 0863, amended, #445): a value-flow of the crate's cells over its bodies' MIR, Steensgaard's unification, a class of each cell only the crate's code reaches, any other what code rust-js can't see may reach, or a constant's value may be, and whether that code, a library's consumer, may reach it (#456); a JS collection's methods keep what they're given in it (ADR 1050) | `analysis/cell_flow.rs` | `CellFlow`, `call_class`, `classes_at` | |
| What a call, std's code's too, and a cell's own impls may borrow (ADRs 0580, 0863), a fn pointer's of what it may be (ADR 0907), and which cells may be one, a `dyn` any type only where made in place, and a thread-local's own cell only its key reaches no other (ADR 0960, amended); a promise's resolve given what runs nothing, nothing, its callee the first parameter of a `#[rust_js::calls_with_resolvers]` binding's executor (rd654, ADR 0898, amended) | `analysis/cell_borrows.rs`, what runs none of the crate's code `analysis/cell_borrows/runs_nothing.rs`, what of the crate is a value at hand, one a builtin or the program made `analysis/cell_borrows/at_hand.rs`, what a thread-local the crate alone reaches holds `analysis/cell_borrows/at_hand/slots.rs`, what a local cell its closures share holds `analysis/cell_borrows/at_hand/local_cells.rs`, what std's code runs of the crate's by its bounds `analysis/cell_borrows/std_given.rs`, whether two cells' types may be one cell `analysis/cell_borrows/may_be_cell.rs`, what code that can't be named may borrow of a cell, which a guard held across it asks (#2112 B) `analysis/cell_borrows/unnamed.rs`; `analysis/resolvers.rs` | `call_asks`, `unnamed_may_borrow`, `cell_call_asks`, `bounds_ask`, `call_runs_unseen`, `pointer_call_asks`, `pointer_call_runs_unseen`, `impls_ask`, `may_be_cell`, `unsized_in_place`, `reached_only_by_key`, `resolve_runs_nothing`; `is_resolve` | |
| What a fn pointer may be: each function and closure the crate makes one of its type of, or anything where a value of its type may come from outside (ADR 0907); whether it may only be a Rust function, the crate's or a library's of a field its contract says so of, and which of the crate's fields are such (ADR 1007); which structs a value from outside may be, a JS object the program didn't build, whose field a getter may answer, each read `Known::Getter`, past a struct the program or a library builds, which its contract says (ADR 1022, amended) | `analysis/fn_pointers.rs`; `representation.rs`; `library.rs` | `FnPointers`, `may_be`, `rust_only`, `rust_fields`, `js_may_make`, `built`; `project` | |
| Which of the crate's functions never unwind, read from their MIR, and does a call of std's, read from its MIR as it resolves (#1666), or a `Drop` given a closure that can't panic (#1930); which of std's calls are quiet, none printing, writing a cell or panicking; and does a JS node throw only where its parts do (ADRs 0901, 0902)? | `analysis/unwinding.rs`; `js/expr_walk.rs` | `never_unwinding`, `std_call_never_unwinds`, `never_unwinds_given`, `std_quiet`; `may_throw`, `never_throws` | |
| Where a future's running as it's made would be seen, refused there (ADR 0029, amended, #965), of what's quiet | `analysis/unwinding/eager_futures.rs` | `refuse_where_eagerness_shows`, `awaits_quietly` | |
| Whether stepping an iterator of a type may panic, by what its revealed type runs: its closures and functions, the crate's, a library's or a `dyn` iterator behind std's adapters, whose steps no JS shows, and what std's iterators do themselves, the items and exhausted parts they drop and the values they clone, recognition's one catalog of it (ADRs 0465, 1089, amended 2026-11-04) | `mir/iter/collect.rs`; `recognition/iterators.rs` | `steps_may_panic`, `call_may_unwind`; `stepping_work` | |
| Whether a clone or a `Default` of a type may panic, and which of a value's parts rust-js makes in turn a later part's making may unwind past, dropped as it does, `$madeInTurn` and `$madeEach` (ADR 1102) | `std_impls/in_turn.rs`, `std_impls/defaults.rs` | `clone_may_panic`, `default_may_panic`, `held_in_turn`, `parts_in_turn` | |
| How an item is written over by a clone of another, as std's `clone_from` does it: its own, a type parameter's dictionary's, or a `Vec`'s, std's `SpecCloneIntoVec` (`$cloneIntoVec`), asked by `fill`, `clone_from_slice` and a `Vec`'s `clone_from` (ADRs 0507, 1102 amended, #919) | `std_types/slice.rs` | `clone_over`, `vec_clone_into` | |
| A type equal only to itself, a JS key by itself, a library's as its contract says (ADR 0864) | `std_impls.rs`; `library.rs` | `equal_only_to_itself` | |
| What each comparison runs, from rustc's impls, never one for another (ADR 0057, amended): `partial_cmp` its own, its `cmp` or its parts'; `<` and the rest JS's own, an impl's own `lt` or `partial_cmp` against 0; `!=` an impl's own `ne`, a type parameter's dictionary's, or std's that asks its parts' (#937, #944, #962); the comparison dictionaries' makers, which hold std's provided methods, and std's quiet orders' shared dictionaries | `ordering.rs`; `std_impls/not_equal.rs` | `partial_runs`, `operator_value`, `own_ne`, `ne_value`, `comparison_maker`, `held_by_comparison_dictionary`, `quiet_dictionary` | |
| Std call classification, and the std functions a compile leaves unrecognised, `--without-recognizer`'s, in lowering and every analysis alike (ADR 1128) | `recognition/calls.rs`; `recognition/unrecognized.rs`; `session.rs` | `classify`, and its answers through `shortcuts.rs`; `left`; `recognized` | |
| Std identities and layout | `recognition/std_items.rs`, `recognition/types.rs`, `recognition/fmt.rs`, `recognition/iterators.rs` | `StdItem`, `is_std_type`, `is_std_def`, `std_item`, `trait_method`, `cell_value_field`, `deref_target`, `debug_builder`, `builder_step`, `iterator_method` | |
| Std's items a recognizer knows by their path, by `DefId` (#1681's DefId pass): one table of rows resolved once a session, through std's modules, then `core`'s and `alloc`'s; a type's methods by whose and their name, of rustc's impls; what's under one of std's modules, by visible parents. Never a path's text: `test/architecture-std-paths.test.ts`, a ratchet of what's left (`test/std-path-text.txt`) | `recognition/std_paths.rs`, `recognition/registry.rs` | `StdPath`, `std_path_item`, `is_std_path`, `std_path_id`, `std_inherent_method`, `std_trait_method`, `std_path_under` | |
| What a loop over a range of integers or a slice's items calls of it, `into_iter`, `next`, `rev`, `enumerate`, a slice's `iter`, and an integer's `min` and `max`, for the MIR intervals (#1672 M3, ADR 1137) | `recognition/loop_ranges.rs` | `range_step`, `integer_min_max` | |
| Whether a map's lookup of a key reads the same until the key is written: its `Hash`, `Eq`, order and `Borrow` std's or derived all the way down, over types with no cell, for the key-held facts of THIR and MIR (ADR 0059, #1844) | `recognition/stable_keys.rs` | `key_stays` | |
| What `size_of`, `align_of`, `type_name` and a `TypeId` tell of a type, and which of std's functions ask them (ADR 0145) | `recognition/type_facts.rs` | `TypeFact`, `type_facts`, `layout_query`, `map_of_param`, `class_key` | |
| The js and react crates' items that lowering knows by name: a JS `Set` or `Map`, a class, the registry's symbols, a function's `bind`, `call` and `apply`, a hook's dependencies and a context's members | `recognition/js_items.rs` | `js_set_or_map`, `is_js_class`, `is_symbol_for_def`, `is_function_bind`, `is_dependency_list`, `is_context_member` | |
| Whether a crate is one of std's, `core`, `alloc` or `std`, by the lang item it defines, not its name (ADR 1054), and std's path of a std item, printed once an item a session (ADR 1100) | `recognition/std_crates.rs` | `std_crate`, `std_crate_num`, `is_std_crate`, `of_std_crate`, `std_path` | |
| Std coverage catalog | `recognition/registry.rs` | `measured` | |
| Compiled std bodies, types and module placement; the catalog's rows resolved once a session, a row resolving to nothing an internal error (#1681) | `recognition/compiled_std.rs`, its rows `recognition/compiled_std/rows.rs`, `recognition/adapter_types.rs` | `compiled_std`, `compiled_rows`, `compiled_provided_method`, `is_compiled_std`, `is_compiled_std_type`, `is_compiled_std_iterator`, `std_home_module`, `memory_intrinsic`, `addresses_memory` | |
| Memory-addressing operations, and which two handles compared are two objects compared (ADR 0285, amended) | `pointers.rs` | `raw_pointee`, `mir_addresses_memory`, `same_object` | |
| Which types' objects say where they're stored, of a crate that asks an address, and where each is recorded, at each borrow of its place; a nested place's root and offset (ADR 0562, amended; #1089, #1110, #1113) | `analysis/stored_places.rs`; `mir/stored.rs` | `StoredPlaces`, `records`; `record_stored`, `stored_handle`, `place_in_root`, `rooted_from`, `read_more_than_once` | |
| Where a value keeps what it holds inline where JS holds that as another place: what each `{ value }` container std lends from holds and its offset, by std's field names and rustc's layout; where in a value the object its JS is sits, an `Option`'s payload past its tag, a `RefCell` of an object's value; what's an allocation of its own; the record where std's call is given a container, the pointer to what holds an object, and the refusal of std's other lending functions and of generic code whose instances disagree (ADR 0562, amended, #1114) | `lent.rs` | `held_inline`, `object_within`, `known_within`, `stored_within`, `object_pointer`, `record_lent`, `property_record`, `record_inserted`, `check_lent`, `variant_field_offset`, `place_field_offset` | |
| Std iterator specialization | `recognition/specialization.rs`, `recognition/compiled_std.rs` | `fact_specialization` (the one a fact picks), `optimized_default` (the one that only does less work), `std_private` | |
| Whether std folds a zip to its shorter side's count of pairs, otherwise than its `next()` steps it, and whether that shows: what a step clones, the one catalog `stepping_work` reads too, and whether a clone runs the crate's code (ADRs 0973, 0997, amended 2026-11-10); whether stepping an iterator runs code a program sees, a closure's, a function's or a `dyn Fn*`'s, the one answer of what std's shortcuts leave unrun, of the types an `impl Trait` hides (#1225) | `recognition/zip_folds.rs` | `zip_folds_to_the_shorter`, `folds_apart`, `stepping_clones`, `steps_run_code` | |
| What an iterator owns of what it gives, which its drops drop and which nothing else holds (#141) | `recognition/owners.rs` | `owned_items`, `owned_into_items`, `owns_one_item`, `gives_owned_items` | |
| Compiled iterator stepping | `iterators.rs` | `std_iterator_steps`, `compiled_step` | |
| Which iterator chains are transparent, each's first reason it isn't, counted where decisions are asked; a measurement nothing reads yet (ADR 1136, amended); the iterator methods whose hand-written form the tables prove, `PROVED_LINKS` | `iterators/transparent.rs`; `recognition/compiled_std.rs` | `count_chains`; `is_proved_link` | |
| Whether a step of std's, an `OwnStep` (`fold`, `rfold`, `try_rfold`, `nth`, `nth_back`, `last`, `count`), of an iterator reaches the one the crate's own iterator writes, through what each std adapter asks of what it holds, or may, of a type parameter, `Reach`; and what each adapter's step asks of what it holds, the one catalog of it (#505, #521, #697, ADR 1089) | `recognition/own_steps.rs` | `reaches_its_own`, `OwnStep`, `Reach`, `asked_sources`, `folds_by_its_own_nth`, `nth_folded_sources` | |
| std's steps, the one catalog of them: which of `Iterator`'s, `DoubleEndedIterator`'s and `ExactSizeIterator`'s methods each std adapter and wrapper writes, and what each asks of what it holds, made from std alone by `test/std-steps.test.ts` (rust-src's impls and a native run of `test/std-steps.rs`) into `std_steps.txt`; and what std's `&mut I` and `Box<I>` pass on to the iterator they hold, read from it (#2000, #2112) | `recognition/std_steps.rs` | `Wrapper`, `Passed`, `passes`, `own_methods`, `writes` | |
| Which of std's steps of an iterator reach a source that drops what it passes as one slice, a `Vec`'s `IntoIter`'s `advance_by` and `advance_back_by`, and what std's `fold` asks an `nth_back` of, each followed through `own_steps.rs`'s catalog (#234, #668, #670, #697) | `recognition/slice_steps.rs` | `nths_a_slice`, `nths_a_slice_in_order`, `nth_back_folded_sources` | |
| Who drops an iterator std's code takes by value, `collect`'s, `extend`'s, `unzip`'s, `eq`'s: its consumer closing it, or dropped after it, never both (#1116, #1120, #1123) | `mir/iter/closing.rs`; `mir/iter.rs` | `consumer_closes` and `consumer_closing`, whether its consumer closes it; `dropped_after`, whether it's dropped after; `consumed_call`, which drops it, of `mir_iter_call`'s and of a std combinator's, `consumed_by_std` | |
| Where a `Vec`'s value is given its slice's steps, `$slicing`, as its type is lost: generic code's, std's compiled adapters', a step's helper (#703, #704) | `mir/iter/closing.rs` | `slicing` | |
| Generic IntoIterator representation | `iterators.rs`, `recognition/iterators.rs` | `into_iter_held_as_it_is`, `into_iter_as_it_is`, `into_iter_by_dictionary` | |
| Function-value mutable arguments | `mut_refs.rs` | `unboxed_by_function` | |
| What a type parameter is given for a `&mut` to a function or an iterator, the value itself where it's only called or stepped, read through each boxed `&mut` (#992, #1909) | `mut_refs/given.rs` | `given_as_the_value`, `called_value`, `called_box_escapes` | |
| Body module placement | `lower.rs` | `body_module` | |
| A crate's `#[test]`s, which module's test file registers each, and which module of tests is its parent's test file (ADR 1076); what each runner's `test` is imported from | `analysis/tests.rs`; `settings.rs` | `collect_tests`; `Runner` | |
| Which `const`s written in a function's body are its, a `const` there, and what reads each (ADR 0031, amended) | `analysis/naming.rs` | `local_consts` | |
| Which library functions a module's `use` marked `#[rust_js::require]` reads through `require(module)` where it reads them, each other `use` so marked refused (rd684, ADR 0305, amended); `fn_ref` reads them so | `analysis/naming.rs`; `items.rs` | `required_uses`; `fn_ref` | |
| A parameter's JS name, its body's and its declaration's, and a fn pointer's written one (ADR 1075) | `parameter_names.rs` | `parameter_name`, `written_parameter_name` | |
| What a function's JS takes and gives as a JS caller calls it, its `JsSignature`: its Rust parameters but a `self` it takes as `this` or not at all, what it takes after them in `evidence_slots`' order, which `Option` is given `null`, a rest parameter, which trailing ones may be left out, a nullable result, and the first of it a JS caller can't give; every declaration path prints it (ADR 1091) | `signatures.rs`; `classes/receivers.rs` | `JsSignature`, `js_signature`, `ungiven`, `optional_from`, `js_parameter_name`; `receiver` | |
| Which function a JS caller can't call as declared, left out with a note; and which traits are declared as the type of their dictionaries, of what entries, and their impls' dictionaries (ADR 1077, amended) | `declarations/functions.rs`, `declarations/dictionaries.rs`; `traits.rs`, `traits/words.rs` | `leave_out`, `trait_interface`, `impl_dictionary`, `dictionary_type`; `method_dictionary`, `bound_word` | |
| What a `&mut` and a `dyn` in a declared signature are, a box `{ value: T }` or the object, a pair `{ value: unknown; impl: Shape<unknown> }` or `any`: lowering's answers, asked in a context of the item declared (#619, #620, ADR 1077, amended) | `declarations.rs`, `declarations/dictionaries.rs`; `representation/mut_ref.rs`; `traits/dyns.rs` | `pair`; `SignatureForms`, `ts_type`; `dynamic_trait` | |
| A declared function type's parameter names, a fn pointer's as written, a `dyn Fn` field's as every literal fills it and a static's or a constant's as the functions its value may be, what names nothing saying nothing (#630), and which trailing parameters may be left out (ADRs 0443, 1075, amended) | `declarations/functions.rs` | `optional_from`, `filled_names`, `fill_value_names`, `agreed_names`, `written_function_names`, `function_names` | |
| Std body capture | `analysis.rs` | `collect_std_bodies` | |
| Dictionary cache names | `names.rs` | `dictionary_cache` | |
| String offsets in JS units | `mir/offsets.rs`; `std_types/text.rs` | `in_units`, `call_in_units`, `step`, `cut`, `seed`; `found`, `sliced_in_units` | |
| A shadow that is the binding it shadows | `mir/tidy/shadows.rs` | `shadowed` | |
| What a JS tree writes of its variables, a binding replaced, an object changed, a handle made, each in a closure too (ADR 0670); and whether, and how often, it sets one, `x++` and a handle among the ways (ADR 1088, #523); and which places it writes, each target of a destructuring assignment too (ADR 1088, amended, #1570), which no other module searches a tree for (`test/architecture-writes.test.ts`) | `js/writes.rs` | `Writes`, `sets`, `count`, `places`, `may_replace`, `by_closures` | |
| JS's scoping: each scope's declarations, each read's declaration or a name from outside and whether it's a global's; whether a rename keeps each read (ADR 0357); trees the same but for their declarations' names (ADR 1145) | `js/scopes.rs` | `Tree`, `of`, `of_module`, `of_body`, `renames`, `rename`, `alpha_eq` | |
| A place a tree reads, as a key: a variable's or `this`'s member chain, what a call of it gives, what a spread of an array reads; its root a variable by its id, its fields property keys; none of a chain with a computed member, an index or a call (ADR 1150) | `js/path.rs` | `Path`, `PathRoot`, `path`, `called_path` | |
| Each import's spelling: a crate import's `Name` (`Name::imported`, one per import, each read of it that `Name`) spelled apart from every name its module binds or reads, its lowering's reserved words and the imports before it, `useState$1`, in the order link asked for them; then a local a runtime helper's read is in the scope of given apart from it (#1975). A JS module's import and a helper's keep their words, locals giving way. Link decides only what's imported and reads no word (architecture test). [ADR 1150](decisions/1150-a-variable-is-an-id.md) | `src/prepare/import_names.rs`, first in preparation; `link.rs`, which gives each import its name | `Name::imported`, `js::ImportNames` | |
| What a tree must be before it's printed: each read resolved, to a declaration in scope, an import, the runtime's helper or a global `Expr::global` made (`Name::global`), and no global's read of a name a declaration in scope takes; a name declared once in a block; each jump landing; each module's tree checked in every build, refused naming the pass that broke it, each body between tidy's stages where `stages_checked()`, a debug build or `RUST_JS_VALIDATE`'s (ADR 1143) | `js/validate.rs`; `js/make.rs`; `prepare.rs` | `module`, `body`, `Problem`, `stages_checked`; `global`; `breaking_pass` | |
| What each fact an expression carries is to its readers, a permission, how the program wrote it, a warning or a form the lowering chose; and the facts a compile forgets, `--without-known`'s, every read of a `Facts` of what isn't forgotten, a contract's check of all (ADR 1127) | `js/known.rs`; `js.rs` | `Known`, `Role`, `role`, `forget`, `remembering`, `found_names`; `Facts`, `only_none` | |
| What a body's `let`s and patterns bind, by name, for tidy (`tidy::LetNames`): the `let`s the program wrote, `let mut`'s, one that ends a borrow or holds a frozen one, a drop guard's, and an `if let`'s or let chain's binding named as the field its scrutinee reads, by Rust names, which alone a chain's binding read again in its place may be (ADR 0866, amended, rd226, rd706) | `mir/patterns/let_names.rs` | `let_names` | |
| How each of a body's `Option` locals holds its `None`, decided once: made `null` where it's made, and what each read may trust, never `undefined` alone of one made `null` (ADR 1096, amended, #2112) | `mir/none_forms.rs` | `NoneForms`, `of`, `made_null`, `read` | |
| Which `Option` locals hold `null` for `None` at each write, an input of `NoneForms`: each write a `Some(..)`, a `None` or a call, each read a nullable field's or parameter's, never a parameter, which holds what its caller gave (ADRs 0275, 0781, 0841, #1248) | `mir/patterns/null_nones.rs` | `mir_null_nones`, `tested_some` | |
| Does a jump MIR shared after an `if` chain go to each arm that goes on (ADR 0745, amended)? | `mir/tidy/jumps/continues.rs` | `jumps_into_branches` | |
| How do tidy's jump passes read a tidied body: does a test say an item `$next` gave is none or there, which `if` the program wrote of a jump is a test of, where is the first of its statements written, do they end making a result, and what does a `for` bind of its item, read by its body's first statements? | `mir/tidy/jumps.rs` | `absent`, `present`, `jump_tested`, `first_written`, `made_at_ends`, `binding` | |
| Is an `if` one the program wrote with an `else`, each branch's statements in one of its branches, and which way round MIR turned them: a value written with an `if`, not with the `&&` or `\|\|` MIR makes the same branches of (ADR 0879)? | `mir/tidy/jumps/written.rs` | `written_branches`, `written_order` | |
| Is a JS name a temporary MIR made, `_5`, or one given apart from one, `_5$1`, not one the program names (#1681 change 3)? | `mir/tidy/bindings.rs` | `is_temporary` | |
| Where a variable is JS's `this`, a `with_this` closure's and a class's method's `self`, read through a `const` in a function made in its body, and whether a body reads its own `this` (ADR 0972) | `js/stmt_walk.rs` | `read_as_this_of`, `reads_this` | |
| Which functions read `js::arguments()` or `js::this()`, each read where its JS isn't its caller's refused, and each use of one that reads `this` but to call it (rd332, rd683; ADRs 1048, 1081) | `analysis/arguments.rs` | `reads_arguments`, `reads_its_call` | |
| A `String` appended to twice | `std_types/text.rs` | `appended` | |
| Runtime helper identity and properties | `runtime.rs` | `exports`, `helper_of`, `is_pure`, `names_a_place` | |
| A function's rest parameter and its calls' rest arguments | `bindings.rs`; `calls.rs` | `marked_rest`, `takes_rest`; `rest_given`, `spread_rest` | |
| Whether an arrow that only passes on what it's given is the function it passes on to, by where it's given: an array method's callback (ADR 0279), an event's handler (ADR 0198), a binding's `FnOnce` callback (ADR 0262's exception), a closure that captures nothing as a `fn`, an arrow rust-js makes of a function as a value (ADR 0605, amended), and a closure that passes on what it took by value, then its parameters, the crate's function bound, `g.bind(null, a)`; a namespace's method that only passes its parameters on, the function re-exported (ADR 1037, amended); a free function that only passes its parameters on to a function at one instantiation, the function re-exported where its JS is that call alone (ADR 1064); a closure that's the function it calls where it's made and held by a `let`, and one that passes all on to a namespace's function through a value (ADR 0605, amended); a `dyn Fn` parameter each of the crate's calls fills with its own function, of a function only calls reach (ADR 0605, amended, rd616); the shape, a declaration's too (ADR 0610), and a function's of a module constant's field that reads no `this`, the value taken apart (ADR 0610, amended, #533: `prepare::forwarding::fields`, `Known::ThisUnread`, `Pointers::call_facts`), and a private constant only called through, a namespace's methods (#561, #584: `prepare::forwarding::members`, `Known::NamespaceMethod`); and what each closure MIR makes passes its call on to, its one fact | `passed_on.rs`; `js/stmt_walk.rs`; `mir.rs` | `passed_on`, `Taker`, `Forwarding`, `forwarded_to`, `forwarded_function`, `forwarder`, `forwarded_call`, `counted_closure`, `held_functions`, `binds`, `bound`, `own_callbacks`; `passes_on`; `forwarding` | |
| The order a struct literal's object lists its fields, and reads them, in | `aggregates.rs` | `written_order` | |
| A variant without fields as a body writes it, a numbered enum's its module's `const` (ADR 0722), and a constant rustc computed there | `representation.rs` | `unit_variant`, `const_value` | |
| Whether a body reads a `const` or a numbered enum's variant by its name, or, where its module loads, another module's as its value (ADR 0722) | `items.rs` | `reads_by_name` | |
| What the crate's modules import of each other where their JS loads, through the JS files of the program's own they import too, which a module's load and a dictionary in a load cycle ask (ADR 0031, amended) | `load_order.rs`; `program.rs`; `output/local_js.rs`; `to_oxc/loads.rs` | `ModuleImports`, `reaches`; `JsModules`, `loaded`; `LocalJs`; `loaded_modules` | |
| Whether a struct literal's object is across lines, as the literal is written | `aggregates.rs` | `object_layout`, `const_object` | |
| An untagged enum's `undefined` and `null`, and what's there of `classify(Some(v))` | `untagged.rs`; `body_queries.rs` | `unit_value`, `holds_nullish`, `told_by_kind`; `given_present`, `present_calls` | |
| What an untagged enum's variants are together, a test's and a `switch`'s case, and its objects', `kind.object()`'s (ADR 0910) | `untagged.rs` | `untagged_test`, `untagged_cases`, `object_test` | |
| Which `From` into an untagged enum is the value it's given, the crate's whose body is the variant of its argument, a library's its contract lists, a binding crate's, held to that where `--rustc` compiles it; any other's body called (ADR 0214, amended, #1775) | `untagged/froms.rs` | `from_gives_its_argument`, `gives_its_variant`, `calls_from_giving_its_argument`, `binding_froms_give_their_variants` | |
| An untagged enum's method that reads the same field of each variant's value, lowered before its callers, each call of it that read of its value where its JS is the read of its parameter alone, written only where something reads it as a value (rd317, ADR 0214, amended) | `untagged/field_reads.rs`; `pipeline/field_reads.rs`; `mir/fields_read.rs` | `reads_each_variants_field`, `read_of_its_parameter`; `found`, `first`; `variant_field_read` | |
| What JS couldn't tell apart of an untagged enum, refused before any of it is lowered: variants of one kind, a nullish variant that holds something, a variant of more than one value, a `From` that isn't its variant of its argument (ADR 0214) | `untagged/validated.rs` | `validate` | |
| An untagged enum whose `Display` writes each variant's value as it is, a string, an integer, a `bool` or a `char`, whose plain `{}` is the value in JS's template, `${id}`, from its impl's THIR, a library's by its contract, its impl written only where something calls it (rd730), a `format_args!`'s read where `Debug`'s analysis reads it (ADR 0214, amended, #557) | `untagged/displayed.rs`; `analysis/debug.rs`; `format_spec.rs`; `display/written.rs` | `displays_its_value`, `displaying_impls`; `format_calls`; `format_value`; `display_string_with` | |
| Whether a writer writes its text through a sink, piece by piece, its `Debug` builders and derived `Debug` written as core's `fmt/builders.rs` writes them, each piece as it's taken, by the runtime's model, `debug_builders.js`: one that may print as it formats, or stop part-way where a panic may be caught, or whose builder's part may give `Err`, `fmt_failures`' answer, read; and a std value's `Debug` of a part that may stop, written through the same builders (ADRs 1130, 0187, amended, #1533, #1557, #1561) | `analysis/prints.rs`; `display/streamed.rs`; `display/composed.rs` | `writes_through_sink`, `writes_through_a_sink`; `streamed_builder`, `streamed_derived`; `composed_may_stop`, `composed_debug` | |
| std's split of each value a `write!` writes into a writer of the crate's own or a `W: fmt::Write`'s, a call of the writer's per piece, through `$writerSink`; a writer of the crate's own shown so, which writes through a sink `$writingInto` arms for it, each write of it by its `Formatter`'s options; a float by `flt2dec`'s parts, `$writeFloat`; a composite's `Debug` by core's builders, the runtime's one model (`debug_builders.js`), each part written by a writing function through `$padAdapter` where pretty; whether a `format_args!` is written so, and which `fmt::Arguments` parameters each function writes so, given the function that writes into the sink it claims (`written_params::written_arguments`), only where something may arm a sink (`arms_sinks`); a std formatting trait's `fmt` as a `fn` pointer, the function that writes into a sink (`std_fmt_fn_value`); a library's writer function framed on both sides (`Foreign::frames_writer_fn`); each the one way (ADR 1135, #1652) | `display/sink_writes.rs`; `display/sink_debug.rs`; `mir/std_calls/written_into.rs`; `analysis/prints.rs`; `mir/arguments_use.rs`; `recognition/fmt.rs` | `sink_writes`, `sink_writes_given`, `writes_into_a_writer`, `SinkWrite`; `debug_sink_writes`, `writing_fn`; `arguments_writes`, `written_into_writer`, `written_through_the_sink`; `written_into_writers`; `arguments_use`, `written_into_a_formatter`; `writes_through_write_str`, `writes_arguments_piecewise`, `writes_through_a_dictionary` | |
| A value only a dictionary knows, a `T`'s or a `dyn`'s, written into a writer a piece at a time by its formatting dictionary's `fmtInto`: which type parameters each function writes so, of its own `write!`s and through its calls, a library's by its manifest's `written_into`; the types that reach one, and where a `dyn` is written so every impl of the crate's own and each value made a `dyn`, whose dictionaries carry `fmtInto`; of a library, each value it makes a `dyn` of given best-effort, a dictionary the lowering can't write so left without, which it gives back (`Dependencies::fmt_into_left`) into its contract's `makes_fmt_dyns`, and a consumer's `dyn` written so where a library says it may make one without, refused (#1797, #1802), and a library's type its contract doesn't list, which each contract lists, a bug of rust-js's (#1688); the type a formatting impl is of, through the references, `Box`es and `Pin`s around it, `recognition/fmt.rs`'s `formatted_adt`, and whether a reference or a `Box` formats itself, `Recognition::formats_itself` (#1798); the entry of an impl of the crate's own, `$writingInto` of its writer, and of a std value's, its writes (#1652, ADR 1135) | `analysis/written_params.rs`; `display/fmt_into.rs` | `written_params`, `WrittenParams`; `written_by_dictionary`, `impl_fmt_into`, `std_fmt_into` | |
| Which untagged enums' values are each an array they hold themselves, changed in place, a `&mut` the array and a `RefCell` of one the array it holds; the most items a value of an array's type has, which a write that keeps some writes the others of; and a copy of one changed in place (ADR 1046) | `untagged.rs`; `copies.rs` | `arrays_in_place`, `most_items`; `untagged_changed_itself`, `sliced_untagged` | |
| What `#[rust_js::types]` says a type is to TypeScript, read, or the error saying what it reads | `declarations/written.rs` | `written_types`, `Written`, `validate_types` | |
| What a binding trait's associated type is to TypeScript, `<Target as Listen<Name>>::Event`, and the key a bound of it constrains | `declarations/projections.rs` | `projection`, `indexed_params` | |
| What a type parameter bound by a binding trait that says what it is is to TypeScript: erased to the type, `children: ReactNode`, or its constraint where its identity shows, `T extends Element` (#1866) | `declarations/bounds.rs` | `erasures`, `constraints` | |
| Which parameter a function's `js::Is<T>` guards, proved of its MIR: the one each `js::Is::when`, or a typed test's trusted `js::Is::assert`, of its body is given as passed, declared `unknown`; any other refused (#1828, ADR 1155) | `predicates.rs`; `recognition/js_items.rs` | `guarded`, `predicate_of`; `is_js_is`, `trusted_js_predicate` | |
| Whether an alias's `#[rust_js::types]` writes a type of its own, declared as written and named where it's used, a library's consumer's too (rd538) | `declarations/aliases.rs` | `declares_written_alias` | |
| Which types a module's declarations declare, another module's or a library's consumer importing them, and a library's type's `.d.ts` and name, its contract's (ADR 0210, amended) | `declarations.rs`; `library.rs` | `declares_type`; `declared_type` | |
| A string's count of UTF-16 units, its `length`; and a comparison of two strings' units, which a closure that sorts by is JS's `sort()` (ADR 0945) | `recognition/types.rs`; `body_queries.rs` | `counts_utf16_units`, `compares_utf16_units`; `orders_by_utf16_units` | |
| What a place is the value of, a box's content its local's (ADR 0023), and so whether a statement writes a local, `*b = v` as `b = v`; what each local is assigned once; whether a variable is set or lent, of THIR (#980, #997, #1000) | `body_queries/values.rs`; `body_queries.rs` | `through_box`, `value_local`, `assigned_once`; `changes` | |
| Which plain `Option` fields a `js::null()` is written to, and which cell fields of one, made of it, `set` to it or assigned it through `borrow_mut()`, so declared `\| null` (ADR 0935, amended, rd415) | `body_queries.rs` | `null_written_fields` | |
| Whether a type is a JS class, what each item of its impl is of it, and the module it is in, its constructor's (ADR 0761, amended); whether a binding's trait is a JS class a class that implements it extends, an ES class, and which it is of a class (ADR 1041); whether a trait is a trait of classes, its methods each class's prototype's (ADR 1035), what a call of one of its methods may run, and whether one gives a field, which is that field (ADR 1038); whether one is an object of methods, which they are and the property each is of its literals (ADR 0943); whether one is a module's namespace, its methods the module's functions, its value the module's, `FnCx::namespace_value` (ADR 1037) | `classes.rs`; `items.rs` | `is_class`, `member`, `constructor_of`, `home`; `base_class`, `subclass_impl_class`, `superclass`; `is_class_trait`, `trait_impl_class`, `trait_member_class`, `implementations`, `field_accessor`, `fields_given`; `is_object`, `own_methods`, `object_of`, `property`; `is_namespace`; `namespace_value` | |
| Whether a CSS Module's binding, a `#[rust_js::css_module]` struct's getters, names classes its file exports, the file found and read as the program's own, what the build read; and the keys a CSS Module exports, Lightning CSS's, read by this one function alone (ADR 1133) | `bindings/css_module.rs`; `program.rs`; `output/local_js.rs`; `css_modules.rs` | `refuse_unverified`; `css_module_keys`; `LocalJs`; `exported_keys` | |
| Which closures are written in place where a value JS finds by identity among those given before is given, a `#[rust_js::given_before]` trait's, removal's listener, refused (ADR 1021, amended) | `bindings/given_before.rs` | `reject_made_in_place` | |
| Whether a constructor the program writes is held as a `js::Class`, `Class::of(f)`, and not as a value of any shape, constructed or its prototype read, which is refused (ADR 1009) | `classes.rs` | `reject_constructors_as_values` | |
| The class a binding's type is as a value, the one `instanceof` tests it by where TypeScript names it so (ADR 0911) | `untagged.rs` | `class_value` | |
| How an untagged enum's JS object type's class is told: a DOM node by the member its `#[rust_js::test(nodeType = 9)]` names, in any realm; any other class `instanceof` would tell refused where its test is made, a `match`'s once its arms differ (ADR 1062, amended, #555); webapi's generator writes each node class's from WebIDL | `untagged/classes.rs`; `mir/structure.rs` | `member_of_class`, `untold_class`, `refuse_untold` | |
| What a place stores of what's written, a nullable field's `None` `null`, one nothing reads apart's as given (#665), whether a JS value may be `undefined`, and which places store so, however reached | `js/storage.rs`, `js/make.rs`; `representation.rs` | `Storage`, `written`, `written_alike`; `may_be_undefined`; `nullable_cell_field`, `nullable_cell_handle` | |
| Whether a `Some` of a type parameter is boxed: none is where no call gives it what may look like `None`, a library's said by its `unboxed` fact; a function JS calls takes each as JS gives it, and which copy of a function a call calls, where its crate's Rust needs one apart, and which copies are alike; a function JS alone reads gives nothing (ADR 0051, amended, #565) | `analysis/mutation.rs`; `analysis/js_given.rs`; `instances.rs`; `pipeline/copies.rs`; `representation/options.rs` | `unboxed_params`; `given_calls_of`, `read_by_js_alone`; `JsGiven`, `Params`, `js_facing`, `called`; `settle`; `boxed_payload`, `defined_by_its_bounds`, `may_look_like_none`, `lowered_params` | |
| Which locals hold an `Option` of an `Option` as its payload, `None` `undefined` and `Some(None)` `null`, each read knowing what its writes made, and an inner `Option` read whole only where it's only tested, as given on its `null` would be seen (ADR 1017, amended); a `#[rust_js::nullable]` `Option<Option<T>>` field, WebIDL's optional nullable member, held so wherever it's made, set or read, and the box where it meets code that boxes (#470, ADR 0051, amended); and a nullable field of an `Option` of what's never `null` but may be `undefined`, held as the value, its `None` `null`, which `js::any_of` gives as it's held (ADR 1070) | `mir/nested_options.rs`; `representation/options.rs`; `bindings/nullable.rs` | `mir_nested`, `only_tested`, `nested_field`, `held_nested`, `held_form`, `held_none`, `boxed_of_held`, `held_of_boxed`, `held_any_of`; `nested_payload`, `undefined_payload`, `nullish_as_it_is`; `holds_nested`, `holds_value`, `is_held`, `undefined_never_null` | |
| What JS is given of a `()`: `undefined`, or a list's `[]` | `representation.rs` | `unit_given`, `UnitTo` | |
| A guard `if let Some(v) = value` that holds where its arm of `match js::classify(value)` matched | `body_queries.rs` | `held_guards` | |
| A cell's `get().unwrap()` that can't fail, the first call of the statement after its `set(Some(..))`, its value (ADR 0294, amended) | `body_queries/set_reads.rs` | `set_then_unwrapped` | |
| A binding getter's `unwrap()` read only as the callee of its call or the `this` of a binding's method, the getter's read (rd519, ADR 1066, the user's decision) | `body_queries/getter_unwraps.rs` | `getter_unwraps` | |
| `js::is_defined` and `js::is_function` of a global, a binding's path or member, or a value JS gave, `typeof MessageChannel !== "undefined"` (ADRs 0597, 0942); the path of bindings they and `js::method` read | `mir/calls/defined.rs` | `mir_defined`, `reads_a_value`, `asks_of_value`, `defined_path`, `path_value` | |
| `js::method`'s read of a path from a value, a binding's method or a member by its key (rd181, ADR 1025); a `function::call` of one whose `this` is `Some` of the path's value, the same variable, the method's call, and a key's read only so (rd334, ADR 1025, amended); its `f.call(value, ..)` written `value.f(..)` where the method's read is in place of that variable (`Known::OwnMethodCall`), and of a key's (`Known::KeyedMethod`) with a `let` the program wrote that only that call, the next statement, reads, the `let`'s value in its place (rd226b, ADR 0866, amended; the decision `tidy/moves.rs`'s) | `mir/calls/method_paths.rs`; `body_queries/method_calls.rs`; `js/make.rs`; `mir/tidy/moves.rs` | `mir_method`, `mir_keyed_method`; `method_calls`; `own_method_call`, `own_keyed_method_receiver`, `own_method_called`; `receiver_reads` | |
| Which `rev()`, `enumerate()`, `map()` or `inspect()` is stepped where it's made, the recognizers', and which is held, std's compiled struct (ADR 0871) | `body_queries.rs`; `mir/iter.rs`; `iterators.rs` | `in_place_adapters`, read by each adapter's maker, which marks what it makes in place (#730); `compiled_struct`, of a value; `made_in_place` | |
| When a collect asks its iterator's `size_hint`, and for what room: std's `SpecFromIter` of a `TrustedLen` one, `SpecFromIterNested` of another, `extend_desugared` as the `Vec` fills, and the reserves of a `String`'s and a hash table's `extend`; the size a grown `Vec`'s items are asked of a caller by, the lowering's and the analysis's one answer (#820); where capacities are kept, which calls grow a `Vec` by how many and by its items' size, as std's growth does (#1180) | `iterators/hint_asks.rs`; `mir/vec_growth.rs`; `recognition/lengths.rs` | `grown_ask`, `grown_by_hint`, `reserved_by_hint`, `grows_by_callers_size`, `grown_vec_item`, `capacity_item_size`, `shunted_ask`, `extended_reserved`, `dictionary_reserved`, `dictionary_grown`, `extended_grown`, `extend_fills`, `grows_as_it_extends`, `extends_as_it_fills`, `refuse_unknown_capacity_extends`; `grown_std_call`; `lower_bound_is_stds`, `grows_by_exact_len`, `hints_no_lower_bound` | |
| What an iterator's value is made as, `$iterator`, `$given`, `$knowing`, closed by `$closing` or lent by `$lent`, written once with the `Known` facts its JS carries, which readers ask of it, never its shape (ADR 1092, amended, #730) | `iterators/value.rs` | `IterValue`, `Form`, `emit` | |
| What an iterator at the JS boundary is: of a JS-facing function's type parameter bound by `Iterator`, `DoubleEndedIterator` or `ExactSizeIterator`, and of an `impl Iterator` it gives back, its `Item` and whether it knows its back and its length; what its declarations print, what its parameter takes, and whether its own JS converts what it gives back, the crate's Rust calling a `Revealed` copy (ADR 1169, #1993, #1994, #1999) | `iterators/boundary.rs`; read by `declarations.rs`, `analysis/js_given.rs`, `mir/structure.rs` | `of_param`, `of_opaque`, `returned`, `converts`, `returned_to_js`; `boundary_ts` | |
| What the crate's later uses ask of a value, by its maker's MIR identity: its back, lent or given, and its length, counted by its maker or asked of a variable a `Vec`'s growth reads (`stepped_locals`, `BackStepped::asked_of`, ADR 1170), and whether a value made before already has a back, by its form down to what its type says (#814, #851, #1795) | `iterators/back_facts.rs` | `asked_of`, `stepped_from_its_back`, `local_stepped_from_its_back`, `making_into`, `knows_its_back_of` | |
| What a collect asks of its type parameters (ADR 0145): how std collects into a `Vec` what its caller gives, `IVecCollect`, the buffer it may collect in in place, `IInPlace`, the size its hint grows by, and of a trait's default, each impl's that copies it; the type facts' own module, `analysis/type_facts.rs`, asks it | `analysis/type_facts/collects.rs` | `collect_asks` | |
| What a compiled std struct a call is given as it's held asks of its caller: what lowering's `knowing_held` steps it by, `KNOWING_STEPS`, of the arguments the callee's iterator bounds step, not the struct whose own method it calls (ADR 0543, #2042) | `analysis/type_facts/knowing.rs` | `knowing_asked` | |
| What one function's ask asks of its callers (ADR 0145): a fact of a slot, of a type built of its slots, or one its trait's dictionaries carry, said in a caller's terms (`asked_of_caller`), and kept as the fixpoint keeps it (`ask`) | `analysis/type_facts/relay.rs` | `Asked`, `ask`, `asked_of`, `asked_of_caller` | |
| What generic code asks the hint of, `size_hint()`, of what its callers give and what a library's generic function is given: each instance over an array's items made knowing it, the back's makers' form or `$taking`, `$skipping`, `$stepBy`; what a generic caller makes of its own parameters and gives on, made so in that caller alone (ADR 0170, amended, #1236, #1240) | `analysis/hint_asked.rs` | `of_crate`, `HintAsked`, `Hinted`, `mark_hint_knowing`, `mark_made_in` | |
| An index or a slice's range in bounds, a key a map's test shows it holds, and a `OnceCell` still empty where it's set, each MIR's intervals' proof (#1672, M4 deleted THIR's ranges) | `analysis/intervals/`; `mir/bounds.rs` | `call_bounds_proved`, `bounds_proved`, `empty_set_proved`; `call_in_bounds` | |
| A join of MIR to THIR by a span, rustc's only key: the one THIR node of a kind there or none, which each join that licenses leaving out a check asks (#1671, #1672) | `body_queries/span_join.rs` | `the_one`, `the_call_at` | |
| Integer intervals of a body's locals, a forward dataflow over the MIR the emitter reads, which decides each bounds check, call index, wrap and cast; `--intervals-shadow` lists what it proves, a `proved-<kind>` line each (#1672, ADR 1137) | `analysis/intervals.rs` | `proofs`, `shadow_check` | |
| What a statement may write, the one answer the MIR intervals' facts are kept by: a local assigned, or reached by a pointer a write goes through, a call or a drop is given, or one that escaped where code not std's runs; a flow-insensitive points-to map of each local's storage and each pointer's pointee, `Freeze` refined to what a shared reference can change, rustc's `MaybeBorrowedLocals`, the escape owner asked; what each write does, replaces, its items, grows, through a typed reference or a cell; what a held `RefCell` guard covers, as `cell_borrows` counts it; and what std's signatures can't say named here alone, `reads_only`, `keeps_vec_length`, `writes_nothing`; run in shadow beside the forgetting rules, both directions a line of `--intervals-shadow`, and asked by the transfer behind `--intervals-by-effects` (#2112 B, ADR 1174; `scripts/effects-shadow.ts`) | `analysis/effects.rs`, a held guard's `analysis/effects/guards.rs`, what std's signatures can't say `analysis/effects/named.rs`, what a type says `analysis/effects/types.rs`; `analysis/intervals/shadowed.rs`, `analysis/intervals/transfer.rs` | `Effects`, `Written`, `of`, `stale`, `changes_through_shared`, `reads_only`, `keeps_vec_length`; `shadow_effects`; `written`, `forget_stale` | |
| What each function only calls reach is given at its calls, each parameter's interval joined over them, callers walked first by the call graph's components (#1672 M2b-2, ADR 1137) | `analysis/intervals/callers.rs` | `parameter_intervals` | |
| What a call tells the MIR intervals of what it gives: std's lengths and `is_empty()`, an integer's `min` and `max`, a range's `into_iter()`, `rev()` and `next()`, a slice's `iter()` and `enumerate()`, text's and a JS string's length, a rest value's `Deref` (ADR 1137) | `analysis/intervals/calls.rs` | `called` | |
| What the MIR intervals prove of a body at its fixed point: each `+`, `-` and `*` in its type's range, each integer cast of a value in its target's, each bounds check that never fails (ADR 1137) | `analysis/intervals/proved.rs` | `proved`, `below_length` | |
| A `Vec` a local holds, or a `&mut Vec` points to, lent `&mut` only to the calls it's given to: its length known, forgotten past a call that may change it, kept past `index_mut` and `deref_mut`, whose slice is of its length (ADR 1137, #1672 M3) | `analysis/intervals/called_on.rs` | `called_on`, `called_on_mut`, `derefed` | |
| What an integer operation gives the MIR intervals: `a op b` of intervals alone where it stays in its integers, at least what a known relation makes it, and whether a product can be JS's `-0` (ADR 1137) | `analysis/intervals/arithmetic.rs` | `arithmetic`, `bare`, `may_make_negative_zero` | |
| A struct's fields a reference parameter is read through, `self.items[self.at]`, locals of the body the walk reads, of a parameter never given on whole (ADR 1137, #1672 M3) | `analysis/intervals/fields.rs` | `fields_as_locals` | |
| What the MIR intervals know of a collection a `RefCell` holds: each borrow and its `Deref` a copy of the cell, one length through all of them, forgotten at a call that may write it, a drop that may run code or a write through a raw pointer (ADR 1137, #1741) | `analysis/intervals/cells.rs` | `cells`, `borrowed_cell`, `may_write_cells`, `forget_lengths` | |
| A `RefCell`'s `borrow_mut()` guard held while it lives, read only by its `deref` and `deref_mut`: its cell's length kept past a call or a drop until it drops, and forgotten past what its `&mut` is given to that may change it (ADR 1137, #1815) | `analysis/intervals/guards.rs` | `guards`, `guard_call`, `guarded`, `guard_lent` | |
| A map's key held, for the MIR intervals: where `contains_key(&k)` held or `m[&k]` gave its value, until the map or the key changes, of a key whose lookup reads the same (`key_stays`) (ADR 1137, rd558) | `analysis/intervals/keys.rs` | `asked_key`, `key_held` | |
| A table's lengths, for the MIR intervals: what `vec![x; n]` is as long as, `n` or another slice, and a table's rows each as long as its row, of a row type no `&mut` in the body may resize (ADR 1137, ADR 0576) | `analysis/intervals/tables.rs` | `rows_kept`, `tabled`, `same_length` | |
| What a closure's captures are where it's made: a value by copy unless its body writes it, what a shared reference refers to, joined over each place it's made, its makers walked first; and its body read with each capture a local of its own, so a test narrows it and a `&mut` of it lends it; and of each closure whose `Some` is a slice's `get` of a capture, what a thread-local's `with_borrow` of it tells its caller (ADR 1137, #1701) | `analysis/intervals/captures.rs` | `capture_intervals`, `as_locals`, `captures_written`, `closure_results`, `made_closure` | |
| The ranges of integers only a `for` loop's `next()` changes, for the MIR intervals: each `&mut` of one, or a reborrow, `next()`'s argument alone, nothing written into its fields (#1672 M3, ADR 1137) | `analysis/intervals/ranges.rs` | `stepped_ranges` | |
| What a session keeps of the MIR intervals: the proofs of each body being lowered, what callers give, and `--intervals-shadow`'s lines (ADR 1137) | `analysis/intervals/session.rs` | `body_proofs`, `given_parameters`, `shadow_check`, `shadow_report` | |
| What a `Vec`'s `push` shows of its length: past it, an item at least, until it's lent `&mut` again or written, its shared reborrows too, never of a local a raw pointer is made of (ADR 1137, #1700) | `analysis/intervals/pushed.rs` | `pushes`, `unpush` | |
| Which `OnceCell`s a body knows are still empty where it sets them, of its MIR: one it made, its holder and the fields to it, through moves, structs and `Rc`s, forgotten where what reaches it is used any other way (ADR 0847, #1672 M3) | `analysis/once_empty.rs` | `empty_sets`, `drop_runs_code` | |
| What the MIR intervals know at a point and how two points join: each local's interval, the comparisons `bool`s hold, copies, lengths, the relations `x < y` and how a join weakens them, a loop's ranges, each slice's least (ADR 1137) | `analysis/intervals/state.rs` | `State`, `Interval`, `below_joined` | |
| The crate's call graph, from the MIR the emitter reads: each body's calls, resolved where rustc can, of a `dyn` or a function pointer anything, and its strongly connected components, callees first; and each body's names, each function it calls or names as a value (`Naming`, #1745), of its generic arguments there, as its THIR names it, or a body of MIR alone its MIR; scanned once, made with what the analysis stands on (`analysis/ground.rs`); each analysis that walks the crate's calls, or asks what a body names, the instances' callers too, asks it (#1671 change 4 M3, ADR 1137, ADR 1148) | `call_graph.rs` | `crate_graph`, `Crate`, `Names`, `names_of`, `call_graph`, `CallGraph` | |
| A slice's `position` only tested and its index taken, JS's `indexOf` or `findIndex`, and an `Option` of an index whose `None` is `-1`, a string's place's too (ADR 1040) | `mir/positions.rs`; `mir/local_facts.rs`; `mir/queries.rs` | `FoundIndexes`, `found_index`; `none_is_minus_one`; `tested_and_taken` | |
| Which `const` names a value a pattern tests, where its `match` or `let` tests it (ADR 0590, amended) | `body_queries/constants.rs`; `named_constants.rs` | `ConstantPatterns`, `pattern_places`, `pattern_at`; `named_symbol`, `named_text`, `name_texts` | |
| Which of an or-pattern's alternatives JS holds as an earlier one, each binding a field at one JS place, and so one arm (ADR 1045) | `mir/alternatives.rs`; `mir/places.rs` | `mir_alike_alternatives`, `one_variant_place` | |
| Which local enum one `match` makes in each arm and the next takes apart at once, each arm going on to its variant's case, its fields locals of their own; and where that `match` is written, whose comments its first case is after (ADR 1055) | `mir/threaded.rs`; `mir/local_facts.rs` | `mir_threaded`; `case_of` | |
| What a binding says it does with what it's given: runs nothing (ADR 0867), only what it's given (ADR 0898), calls it with a promise's resolving functions (rd654), gives back what it was given (ADR 1022), keeps it only in its collection, refused where it can't be said (ADR 1050) | `bindings/effects.rs` | `runs_nothing`, `runs_only_given`, `calls_with_resolvers`, `gives_back`, `keeps_in_collection`, `reject_unkept_collections` | |
| A binding's path, the global it starts from and each property by its name or a quoted key, `window['$RT']`, which a minifier keeps; a path that isn't JS's refused where it's declared (ADR 1033) | `bindings/paths.rs`; `bindings.rs` | `first_name`, `steps`, `read`; `path_refused` | |
| The whole numbers a binding's `f64` is, a WebIDL integer's, `#[rust_js::whole]`; which cast of an `f64` to an integer is the number as it is; and that a call's `f64` is whole, which `{}` shows as JS does (ADRs 0302, 0994, amended) | `bindings/whole.rs`; `body_queries/whole_casts.rs`; `calls.rs` | `whole_range`; `whole_casts`; `given_by` | |
| Which `f64`s are whole, whose JS text is Rust's: of a body, each local each of whose writes gives one; of the crate, each function whose result is one, and each field of its structs each write of which is, no constant holding it, lent only to a `Cell`'s `get`, `set` and `replace`, each `f64` or `Cell<f64>` a closure takes by value, a field of its own, and each thread-local `Cell<f64>` not `pub`, read and written only by its key's `get`, `set`, `take` and `replace` (ADR 0994, amended, rd537, rd653, rd707) | `analysis/wholes.rs` | `Wholes`, `of_crate`, `whole_locals` | |
| A binding's read of a property, which a getter may be, and a chain's test of one | `binding_calls.rs`; `mir/tidy/logicals.rs` | `answered_by_a_getter`, `Known`; `bound_into` | |

**Calls.** `mir/dispatch.rs::mir_call` handles special calls and sends evaluated
arguments to `mir_call_values`. A method std's impl for a `&mut W` or a
`Box<I>` passes on to `W`'s or `I`'s is that method's call, of what the
receiver is to (`mir/dispatch/passed_on.rs`, asking recognition's
`passed_through_mut_ref` and `passed_through_box`). Calls then go to iterator, trait, binding or
crate-function lowering. Recognized std calls go through `mir_std_call`
in `mir/dispatch/known_calls.rs`, which asks each kind in `mir/std_calls.rs`
of what it reads alone and runs the lowering of the one that takes the call
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
`src/to_oxc/trailing.rs` places each comment written after code on its line
in the printed lines, by the map ([ADR 0630](decisions/0630-a-comment-inside-an-expression.md));
`src/to_oxc/placed.rs` places a statement copied into a function from past
it, a `Drop` impl's body, in the function, its comments with it ([ADR 0753](decisions/0753-a-branch-of-only-comments-keeps-them.md));
`src/to_oxc/apart.rs` says where a blank line is between a module's
statements, as the Rust has one ([ADR 0444](decisions/0444-a-comment-is-where-its-code-is.md), amended);
`src/to_oxc/header.rs` owns a module's header text, its imports' and
re-exports' order, and their statements among the code, where their `use`s
are ([ADR 1069](decisions/1069-imports-are-statements-of-the-module.md));
`src/to_oxc/starts.rs` says where each statement and item starts, what a
comment is attached to ([ADR 0444](decisions/0444-a-comment-is-where-its-code-is.md)),
and where a function `prepare` made a declaration elsewhere was, whose
comments are that declaration's (`js::Module::taken_into`, [ADR 0610](decisions/0610-a-forwarding-function-is-its-variable.md), amended);
`src/format.rs` lays it out with oxfmt and remaps changed text; its
`oxc_formatter_core` is `vendor/`'s, with #1269's fix (ADR 1121). Declarations
flow from `src/lower/declarations.rs` through `src/typescript.rs` to `typescript/`.
`src/output.rs` coordinates crate transforms, optional application Prettier,
map adjustment and final artifact assembly.

## Libraries and runtime

A compiled library has JS plus a versioned `src/library.rs` contract embedded
in the host manifest; each fact of it says what makes it true, its scope in
`src/library/scopes.rs`, and, of one true of the library's whole crate
alone, what a consumer that differs does, of which
[library-contract.md](library-contract.md)'s table is made ([ADR 1129](decisions/1129-a-fact-of-a-librarys-contract-says-what-makes-it-true.md)). It carries exported item identities, trait requirements,
held bindings, input fingerprints and crate identity; rustc metadata alone
cannot describe JS linkage. Of a crate whose directory has a `package.json`,
`src/packages.rs` reads its package and what its entry re-exports, by
`to_oxc::entry_exports`, as artifact planning asks; a consumer imports what
the entry re-exports by the package's name where Node finds it and its Rust
names it through the library's root, each module by its own paths (`Foreign::import_key`)
([ADR 1008](decisions/1008-what-a-packages-entry-re-exports-is-imported-by-its-name.md)),
and another module by the package's subpath its `exports` resolves to
that module's JS, `shared/ReactFeatureFlags`
([ADR 0770](decisions/0770-a-librarys-const-is-imported-by-its-name.md), amended). Consumers validate library/compiler identity and
required dependencies. Cargo provides build configuration and caching;
`src/cargo.rs` makes compiler identity visible to Cargo's fingerprints.

`@rust-js/std` contains selected std bodies compiled from the pinned toolchain's
MIR. The catalog in `src/lower/recognition/compiled_std.rs` owns which ones. `bun run std`
regenerates the committed JS and manifest in `std/`; other compilations use
that built-in manifest and import the package. Hand-modeled std operations
remain in their domain modules until migrated.

`src/runtime.rs` owns helper names, dependencies and implementations in
`src/runtime/`; `bun run std` generates `@rust-js/std`'s `std/runtime.js` of
them, beside std's bodies and the package's one entry, `std/index.js`,
which the JS imports both through (#1095). Lowering requests
helpers through `Expr::helper`. Linking collects helper dependencies, and
artifact planning selects imports from the prepared tree's actual reads.
The printer receives those imports; it does not infer them from text.

The runtime's iterators are `$Steps` (`src/runtime/steps.js`), whose
methods are std's defaults of each step. What an adapter passes on of its
source's steps, `fold`, `try_fold`, `rfold`, `try_rfold`, `nth`,
`nth_back`, `last`, `count`, `advance_by` and `advance_back_by`, and what
it specializes, is given by `$forward` (`src/runtime/forward.js`) alone,
which passes on every argument a step takes, an item's drop and an
accumulator's `unwind` ([ADR 1089](decisions/1089-an-iterators-steps-are-passed-on-by-one-combinator.md));
`test/architecture.test.ts` rejects a step assigned anywhere else. Which
of an iterator's steps are its own (`ownNth` and the others) is written by
`forward.js`'s `$owns` and `$ownsAs` alone, and an iterator's lifecycle,
`$state`, by `closing.js`'s `$close`, `$takenBy` and `$stateFrom` alone,
where an impossible move is rust-js's internal error (ADR 1092's #870
amendment; `test/architecture-iterators.test.ts`). What a
helper drops as a panic unwinds is `$unwinds(run, ...drops)`'s alone, the
drops given in std's frame's order, and what a frame's end and drop glue
run in turn is `$inTurn`'s; std's default fold is `$foldBy`'s, after the
value's own step; a drop that throws while one unwinds is
`$dropsUnwinding`'s, which aborts through `$cleanup` (ADR 1092's #817
amendment, ADR 0660's #847 one). What the lowering runs as a panic
unwinds is guarded by `lower/drops/parts.rs` alone: `cleanup`, the one
writer of `$cleanup`, and `dropped_after`, of a drop after a body that runs
as it unwinds too, which is a `finally` only of what can't throw; tidy's
scope merges ask `finally_holds`, and its last pass, "guarded cleanups",
guards each rethrowing `catch` of a body's MIR. A `catch` made outside a
body, a closure's drop of its captures or a std struct's `close`, never
reaches that pass, so drop glue, `dropped_after_unwinding`, guards its
drops where it's made (ADR 0660's #938 amendment). Tidy's contract "what unwinds is guarded",
`test/architecture-iterators.test.ts` and
`test/architecture-cleanups.test.ts` hold it.
`analysis/lazy_locals.rs` supplies all eager keys, including const initializers;
its initializer-effects map is a separate question. The unwinding owner
consumes that set through `analysis/unwinding/local_keys.rs`, resolving only actual
LocalKey with/try_with callbacks with shared MIR queries. Its generic-instance
walk reads the existing function and closure body maps and retains cycle/memo
checks. No downstream phase rebuilds the eager-key policy.
`test/architecture-iterators.test.ts` rejects a runtime `try`, `catch` or
`finally` outside them, but of helpers it names that aren't an iterator's.

### The iterator protocol

[ADR 1092](decisions/1092-the-iterator-protocol-is-one-contract.md) is the
contract every iterator step is checked against. Each rule cites std's
source.

- **Steps.** Each JS step is one std method: `next`, `back`, `fold`,
  `tryFold`, `rfold`, `tryRfold`, `nth`, `nthBack`, `advanceBy`,
  `advanceBackBy`, `count`, `last`, `len`, `hint` and `return()`. A step
  by `&mut` closes nothing. A step by value takes its receiver and drops,
  at its end and as it unwinds, in std's frame order: `f` (`ends`) before
  what's left, the accumulator (`unwind`) first as a step unwinds.
- **`return()`** drops an adapter's fields once, in std's declaration
  order.
- **Which step of what it holds each adapter asks** is std's
  specialization, and the `OwnStep` catalog's (`recognition/own_steps.rs`).
- **The facts a value carries.** Its own steps (`own*`), `dropItem`,
  `held`, `of`, a `Vec`'s slice, a compiled struct's `$steps`, and which
  owner makes and reads each.
- **The lowering's answers for each value.** Who closes it, what it
  holds, whether it slices, whether it knows its back, what it's given.

The structure that enforces the contract has three parts:

- **one runtime primitive** that consumes by value and owns S2's drop order
  (m6-lily, #734);
- **one lowering-side iterator value** that carries each value's facts, so
  no recognizer reads them from `Expr` shapes, spans or runtime properties
  (#730);
- **std's compiled adapters (`--std-iterators`) as the oracle** that the
  hand-written runtime is proved against.

The conformance table (`ITERATOR_TABLE=conformance bun test test/iterator-matrix.test.ts`, ADR 1090's
generator) runs each adapter through each step, in both builds, against
native Rust. The same harness runs the panic points of its cases (#817) and
of operations beyond iterators (`ITERATOR_TABLE=unwind`,
`test/unwind-points.ts`, #854), each by `test/panic-points.ts`'s one runner.

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
