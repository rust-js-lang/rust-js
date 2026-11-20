// What a change to the iterators runs, in each push's check (check.yml's
// `nets` job's `iterators` and `conformance` steps and its `iterator-forms`
// job, at once) and in
// `check:fast` (#731, #766): the slices of the iterator matrix (ADR 1090)
// and of the conformance table (ADR 1092), and every corpus case built
// with std's iterator structs alone (ADR 0908), the user's choice of
// 2026-11-05. What the iterators' code is, `iteratorPaths`, is said
// here alone: test/architecture-iterators.test.ts holds it to
// docs/architecture.md's owners of iterators.
//
//   bun scripts/iteratorSlice.ts run matrix [parts]        the matrix's slice, in parts at once, against native Rust
//   bun scripts/iteratorSlice.ts run conformance [parts]   the conformance table's
//   bun scripts/iteratorSlice.ts run copies [parts]        the copy matrix's slice (test/copy-matrix.ts)
//   bun scripts/iteratorSlice.ts run paths [parts]         the paths table, all of it (test/iterator-paths.ts, #2112)
//   bun scripts/iteratorSlice.ts run forms [parts]         the corpus with std's iterator structs
//   bun scripts/iteratorSlice.ts run rows [parts]          the rows of both tables the change names (scripts/iteratorRows.ts, #818)
//   bun scripts/iteratorSlice.ts run format [parts]        the formatting grid's slice (#941), of a change to the formatting code
//   bun scripts/iteratorSlice.ts run numbers [parts]       the numbers grid's slice (#940), of a change to the numbers' code
//   bun scripts/iteratorSlice.ts run declarations 1        the declarations' TypeScript callers (#822), of a change to what a `None` is at the JS boundary
//   bun scripts/iteratorSlice.ts run async [parts]         the async table (#942), of a change to how a future awaits and drops
//   bun scripts/iteratorSlice.ts run std [parts]           the std net (#1171), of a change to std's collections', slices', strings' or cells' code
//   bun scripts/iteratorSlice.ts run consumers [parts]     the consumer net, of a change to the iterators' code or the drops'
//   bun scripts/iteratorSlice.ts run metamorphic [parts [since]]  each metamorphic rewrite of each corpus case the change adds or edits (#1165)
//   bun scripts/iteratorSlice.ts run memory [parts]        the memory net's slice (#1172), of a change to the pointers' code
//   bun scripts/iteratorSlice.ts run twins [parts]         the twin net's slice (#1205), the runtime's helpers against std's own bodies, of a change to the iterators' code
//   bun scripts/iteratorSlice.ts run twinsAll [parts]      all of the twin net, the nightly's
//
// And a change to the formatting code, `formatPaths`, runs the formatting
// grid's slice (test/format-grid.ts, #941), each push's check's
// `nets` job's `format` step and `check:fast`'s; and to the numbers' code,
// `numberPaths`, the numbers grid's (test/number-grid.ts, #940),
// `numbers`; and to what a `None` is where JS gives or takes it,
// `declarationPaths`, every TypeScript caller of the declarations
// (test/declarations-conformance.test.ts, #822), `declarations`; and to
// how a future awaits and what it drops as it unwinds, `asyncPaths`, the
// async table, all of it (test/async-points.ts, #942), `async`; and
// to the iterators' code or what drops as a panic unwinds,
// `consumerPaths`, the consumer net, all of it (test/consumer-net.ts),
// `consumers`; and to what std's collections, slices, strings and cells
// lower to, `stdNetPaths`, the std net, all of it (test/std-net.ts,
// #1171), `std`; and to the iterators' code, the runtime's among it,
// the twin net (test/twin-net.ts, #1205), `twins`: each a step of the
// `nets` job, named for its net, where `plan` says its change runs it
// (dev-loop change 3). And a change to what the WASM build decides on,
// `wasmPaths`, runs the `wasm` job, and to what rustc's tests run,
// `rustcPaths`, those.
import { existsSync, mkdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { noteRate, pick, type Picked } from "./iteratorRows";
import { lines } from "./parts";
import { caseParts } from "./shard";
import { twinHelpers } from "../test/iterator-matrix";
import { helpersOf, sharedProblems } from "../test/twin-net";
import { machine } from "./shard";

const root = join(import.meta.dir, "..");

/** The iterators' code, as globs of the repository's paths: their
 * lowering, their recognition and the facts of their steps, the runtime's
 * helpers and its package, std's compiled iterators, and the tests these
 * runs are of. */
export const iteratorPaths = [
  "src/lower/iterators.rs",
  "src/lower/iterators/**",
  "src/lower/combinators.rs",
  "src/lower/combinators/**",
  "src/lower/mir/iter.rs",
  "src/lower/mir/iter/**",
  "src/lower/mir/vec_growth.rs",
  // Where a std iterator method's MIR call is lowered (the owner table's row, #2162).
  "src/lower/mir/std_calls/iterators.rs",
  // Whether an iterator is dropped by closing it, `dropped_by_closing` (the owner table's row, #2162).
  "src/lower/drops/parts.rs",
  // What a type parameter read as an iterator is given for a `&mut` to one (#992).
  "src/lower/mut_refs/given.rs",
  // A `for` over the crate's own `IntoIterator`, `user_into_iter` (the owner table's row, #2162).
  "src/lower/loops.rs",
  // A boxed iterator's steps a caller gives generic code, `boxed_steps_given` (the owner table's row, #2162).
  "src/lower/generic_facts.rs",
  // How the runtime steps a range, and an inclusive range an iterator's lent (the owner table's row, #2162).
  "src/lower/std_types/range.rs",
  // A library's contract of what its functions step and the iterators it makes (the owner table's rows, #2162).
  "src/lower/library.rs",
  "src/lower/recognition/adapter_types.rs",
  "src/lower/recognition/iterators.rs",
  "src/lower/recognition/lengths.rs",
  "src/lower/recognition/own_steps.rs",
  "src/lower/recognition/std_steps.rs",
  "src/lower/recognition/slice_steps.rs",
  "src/lower/recognition/owners.rs",
  "src/lower/recognition/specialization.rs",
  "src/lower/recognition/std_items.rs",
  "src/lower/recognition/compiled_std.rs",
  "src/lower/recognition/zip_folds.rs",
  // Owners of the owner table's rows of iterators (#2162): a type known by its
  // bounds as an iterator, a std iterator method that keeps each item, a
  // string's iterators, std's `Iterator` trait (#2174), a range no step
  // leaves exhausted.
  "src/lower/recognition/traits.rs",
  "src/lower/recognition/methods.rs",
  "src/lower/recognition/chars.rs",
  "src/lower/recognition/sorts.rs",
  "src/lower/mir/noted.rs",
  "src/lower/analysis/iterator_steps.rs",
  "src/lower/analysis/type_facts/knowing.rs",
  "src/lower/analysis/back_stepped.rs",
  "src/lower/analysis/hint_asked.rs",
  "src/lower/body_queries.rs",
  "src/lower/body_queries/in_place.rs",
  // What an iterator at the JS boundary is, its readers' (ADR 1169).
  "src/lower/declarations.rs",
  "src/lower/analysis/js_given.rs",
  "src/lower/mir/structure.rs",
  "src/runtime.rs",
  "src/runtime/**",
  "std/runtime.js",
  "std/iter.js",
  "test/iterator-matrix.ts",
  "test/iterator-matrix-prelude.ts",
  "test/iterator-matrix.test.ts",
  "test/iterator-matrix-known.txt",
  "test/iterator-paths.ts",
  "test/std-steps-catalog.ts",
  "test/iterator-forms.test.ts",
  "test/copy-matrix.ts",
  "scripts/iteratorSlice.ts",
];

const globs = iteratorPaths.map((path) => new Bun.Glob(path));

/** The formatting code, as globs of the repository's paths: a
 * placeholder's options and `format_args!`, `Display` and `Debug` and the
 * `Formatter` the crate's own are given, and the runtime's helpers that
 * write, pad and round what they show; and the grid that tests them. */
export const formatPaths = [
  "src/lower/format_spec.rs",
  "src/lower/format_args.rs",
  "src/lower/display.rs",
  "src/lower/display/**",
  "src/lower/recognition/fmt.rs",
  "src/lower/analysis/debug.rs",
  "src/lower/untagged/displayed.rs",
  "src/runtime/debug*.js",
  "src/runtime/display*.js",
  "src/runtime/fmt_error.js",
  "src/runtime/format_float.js",
  "src/runtime/formatted.js",
  "src/runtime/lower_exp.js",
  "src/runtime/pad*.js",
  "src/runtime/zero_pad.js",
  "src/runtime/string_writer.js",
  "test/format-grid.ts",
];

const formatGlobs = formatPaths.map((path) => new Bun.Glob(path));

/** Whether a change of `files` touches the formatting code. */
export function touchesFormatting(files: string[]): boolean {
  return files.some((file) => formatGlobs.some((glob) => glob.match(file)));
}

/** The numbers' code, as globs of the repository's paths: their operators,
 * methods and casts, the methods' recognition, and the runtime's helpers
 * that compute, wrap, check, convert and parse them; and the grid that
 * tests them. */
export const numberPaths = [
  "src/lower/std_types/number.rs",
  "src/lower/std_types/number/**",
  "src/lower/recognition/methods.rs",
  "src/runtime/big_*.js",
  "src/runtime/{add,checked,checked_div,checked_pow,checked_rem,count_ones,div,div_ceil,div_euclid,float_euclid,float_from_bits,float_midpoint,float_to_bits,ilog,int_bits,isqrt}.js",
  "src/runtime/{mul_add,mul_add_f32,overflowing,pow,powf,powi,powi_f32,rem,rem_euclid,round,saturating_pow,signum,trailing_zeros,wrapping_div,wrapping_rem}.js",
  "src/runtime/{rotate_bits,rotate_left,rotate_right,f32_digits,f64_to_big,f64_to_int,parse_big,parse_f32,parse_f64,parse_int}.js",
  "test/number-grid.ts",
];

const numberGlobs = numberPaths.map((path) => new Bun.Glob(path));

/** What a `None` is at the JS boundary, as globs of the repository's
 * paths: the `.d.ts` printer, the one owner of how a place holds a `None`
 * (`NoneForm`) and of what may look like one, the bindings' nullability,
 * and the presence test JS reads it by; and the callers that test them.
 * And each corpus case, each of whose `pub` items' `Option`s the callers
 * are generated of: a case added, `flattened_struct_lent_whole`'s `pub
 * struct Sized`, left the harness's `?Sized` unbuilt, which only the
 * nightly saw. All of it runs, some 22 s, in its own job. */
export const declarationPaths = [
  "src/typescript.rs",
  "src/lower/representation/nones.rs",
  "src/lower/representation/options.rs",
  "src/lower/bindings/nullable.rs",
  "src/js/make.rs",
  "src/js/stmt_walk.rs",
  "test/declarations-conformance.test.ts",
  "test/corpus/*.rs",
];

const declarationGlobs = declarationPaths.map((path) => new Bun.Glob(path));

/** Whether a change of `files` touches what a `None` is at the JS boundary. */
export function touchesDeclarations(files: string[]): boolean {
  return files.some((file) => declarationGlobs.some((glob) => glob.match(file)));
}

/** The pointers' code, as globs of the repository's paths: raw pointers,
 * their arithmetic and std's functions of them, what a `&mut` and a place
 * lent as a pointer are, plain data's bytes, unions, what's held inline
 * where std lends from, and the runtime's helpers that address memory,
 * read and write bytes and views; and the memory net that tests them
 * (#1172). */
export const memoryPaths = [
  "src/lower/pointers.rs",
  "src/lower/pointers/**",
  "src/lower/mut_refs.rs",
  "src/lower/plain.rs",
  "src/lower/unions.rs",
  "src/lower/lent.rs",
  "src/runtime/{pointers,unions,bytes,from_bytes,to_bytes,slice_view}.js",
  "test/memory-net.ts",
  "test/memory-net.test.ts",
  "test/memory-net-known.txt",
];

const memoryGlobs = memoryPaths.map((path) => new Bun.Glob(path));

/** Whether a change of `files` touches the pointers' code. */
export function touchesMemory(files: string[]): boolean {
  return files.some((file) => memoryGlobs.some((glob) => glob.match(file)));
}

/** The facts MIR's intervals keep and what may write past them, as globs
 * of the repository's paths: the intervals, the pointers' code and what a
 * `&mut` is, whose change runs the intervals' variant of
 * test/tidy-stages.test.ts, each case without their proofs (#2112 B). */
export const intervalsPaths = [
  "src/lower/analysis/intervals.rs",
  "src/lower/analysis/intervals/**",
  "src/lower/pointers.rs",
  "src/lower/pointers/**",
  "src/lower/mut_refs.rs",
  "src/lower/mut_refs/**",
];

const intervalsGlobs = intervalsPaths.map((path) => new Bun.Glob(path));

/** Whether a change of `files` touches the facts MIR's intervals keep. */
export function touchesIntervals(files: string[]): boolean {
  return files.some((file) => intervalsGlobs.some((glob) => glob.match(file)));
}

/** The families of test/tidy-stages.test.ts' variants, each run by a push
 * of its own code (scripts/checkFast.ts' `variantFamilies`). */
export const families = ["readability", "intervals"] as const;
export type Family = (typeof families)[number];

/** What drops, and what drops as a panic unwinds, as globs of the
 * repository's paths: the drops and their unwinding's owners. */
const dropPaths = ["src/lower/drops/**", "src/lower/analysis/unwinding.rs"];

/** How a future awaits and what an async body drops as it unwinds, as
 * globs of the repository's paths: `.await`'s lowering, the drops and
 * their unwinding's owners, and the table and its runner. */
export const asyncPaths = ["src/lower/mir/awaits.rs", ...dropPaths, "test/async-points.ts", "test/panic-points.ts"];

const asyncGlobs = asyncPaths.map((path) => new Bun.Glob(path));

/** Whether a change of `files` touches how a future awaits and drops. */
export function touchesAsync(files: string[]): boolean {
  return files.some((file) => asyncGlobs.some((glob) => glob.match(file)));
}

/** What std's consumers do with the iterators they're given, as globs of
 * the repository's paths: the iterators' code, the drops', and the net
 * that tests them, test/consumer-net.ts, and its coverage of std. */
export const consumerPaths = [...iteratorPaths, ...dropPaths, "test/consumer-net.ts", "test/consumer-net.test.ts"];

const consumerGlobs = consumerPaths.map((path) => new Bun.Glob(path));

/** What std's collections, slices, strings and cells lower to, as globs of
 * the repository's paths: their lowering, the runtime, the drops', and the
 * net that tests them, test/std-net.ts, and its coverage of std (#1171). */
export const stdNetPaths = [
  "src/lower/std_types/**",
  "src/lower/std_impls.rs",
  "src/lower/std_impls/**",
  "src/lower/combinators.rs",
  "src/lower/combinators/**",
  "src/lower/mir/std_calls.rs",
  "src/lower/mir/std_calls/**",
  "src/runtime.rs",
  "src/runtime/**",
  "std/runtime.js",
  ...dropPaths,
  "test/std-net.ts",
  "test/std-net.test.ts",
];

const stdNetGlobs = stdNetPaths.map((path) => new Bun.Glob(path));

/** Whether a change of `files` touches what std's collections, slices,
 * strings and cells lower to. */
export function touchesStdNet(files: string[]): boolean {
  return files.some((file) => stdNetGlobs.some((glob) => glob.match(file)));
}

/** Whether a change of `files` touches what std's consumers do with the
 * iterators they're given. */
export function touchesConsumers(files: string[]): boolean {
  return files.some((file) => consumerGlobs.some((glob) => glob.match(file)));
}

/** What the crates of crates.io's corpus are compiled by, as globs of the
 * repository's paths: the compiler, its runtime and std's own crate, and
 * the corpus's list, probes and locks. A change to any of them runs the
 * crate corpus in each push's check (check.yml's `nets` job's `crates`
 * step) and in plain
 * `check:fast`: 9353847cd refused itertools, which only the nightly saw
 * (#1188). */
export const cratePaths = ["src/**", "std/**", "vendor/**", "Cargo.lock", "docs/crate-corpus.md", "scripts/crate-corpus.ts", "scripts/crate-corpus/**"];

const crateGlobs = cratePaths.map((path) => new Bun.Glob(path));

/** Whether a change of `files` touches what compiles crates.io's corpus. */
export function touchesCrates(files: string[]): boolean {
  return files.some((file) => crateGlobs.some((glob) => glob.match(file)));
}

/** What compiles React's port, as globs of the repository's paths: the
 * compiler, its runtime, std's crate, the tooling that builds an app and
 * the bindings its crates use. A change to any of them regenerates the
 * port at its pin in each push's check (check.yml's `nets` job's `port` step),
 * failing on any file that differs: ba69090b3's port diff went unseen
 * until the nightly (#1188, #1189). */
export const portPaths = ["src/**", "std/**", "vendor/**", "tooling/**", "builtins/**", "react/**", "webapi/**", "Cargo.lock", "ports/react-in-rust.pin", "scripts/react-in-rust.ts"];

const portGlobs = portPaths.map((path) => new Bun.Glob(path));

/** The exit of `check` and `regen` where a crate of the port doesn't
 * compile, apart from `regen`'s 1, where its JS differs: a crate that
 * doesn't compile writes nothing to differ. */
export const PORT_DOES_NOT_COMPILE = 3;

/** Whether a change of `files` touches what compiles React's port. */
export function touchesPort(files: string[]): boolean {
  return files.some((file) => portGlobs.some((glob) => glob.match(file)));
}

/** The paths, file or directory, wasm/prebuilt.sh's `INPUTS` says decide
 * rust-js.wasm, the one list of them, as globs of the repository's paths. */
export function wasmInputs(script = readFileSync(join(root, "wasm/prebuilt.sh"), "utf8")): string[] {
  const listed = /^INPUTS=\(([^)]*)\)$/m.exec(script);
  if (!listed) throw new Error("wasm/prebuilt.sh has no `INPUTS=(...)`, which says what decides rust-js.wasm");
  return listed[1]!.trim().split(/\s+/).map((path) => {
    const full = join(root, path);
    return existsSync(full) && statSync(full).isDirectory() ? `${path}/**` : path;
  });
}

/** The files of the checkout `files` import, they among them, and what
 * those import, transitively: a job's own code, whose change is the job's. */
export function importsOf(files: string[]): string[] {
  const seen = new Set<string>();
  const walk = (file: string) => {
    if (seen.has(file) || !existsSync(join(root, file))) return;
    seen.add(file);
    // Each `import .. from`, `export .. from`, bare `import "./x"`, and
    // `await import("./x")`, a module a script loads as it needs it.
    const text = readFileSync(join(root, file), "utf8");
    for (const [, from, bare, loaded] of text.matchAll(/^(?:import|export)[^"';]*?from\s*["'](\.{1,2}\/[^"']+)["']|^import\s*["'](\.{1,2}\/[^"']+)["']|\bimport\(\s*["'](\.{1,2}\/[^"']+)["']\s*\)/gm)) {
      const base = join(dirname(file), (from ?? bare ?? loaded)!);
      walk([base, `${base}.ts`, `${base}.js`].find((path) => existsSync(join(root, path)) && statSync(join(root, path)).isFile()) ?? base);
    }
  };
  files.forEach(walk);
  return [...seen].sort();
}

/** What the check's `wasm` job builds and runs (check.yml): rust-js.wasm,
 * of wasm/prebuilt.sh's inputs; and the snapshots and the playground it
 * compiles, of std, the bindings and the examples, with its tests and
 * what they import; and the workflow. A change to none of them, a doc's or
 * a corpus case's, leaves the job's answer as it was. */
export const wasmPaths = [
  ...wasmInputs(),
  ...importsOf(["test/snapshots.test.ts", "test/playground.test.ts"]),
  "wasm/**",
  "std/**",
  "vendor/**",
  "builtins/**",
  "react/**",
  "webapi/**",
  "serde/**",
  "typescript/**",
  "examples/**",
  "Cargo.lock",
  "package.json",
  "bun.lock",
  "test/snapshots/**",
  "test/*.rs",
  "test/imports/**",
  ".github/workflows/check.yml",
];

const wasmGlobs = wasmPaths.map((path) => new Bun.Glob(path));

/** Whether a change of `files` touches what the `wasm` job builds or runs. */
export function touchesWasm(files: string[]): boolean {
  return files.some((file) => wasmGlobs.some((glob) => glob.match(file)));
}

/** What rustc's tests run (rustc-tests.yml, ADR 0089): the compiler, its
 * runtime and std's crate, the toolchain, the suite's script and what it
 * imports, the known lists and the workflows. A change to none of them
 * leaves their answer as it was. */
export const rustcPaths = [
  ...importsOf(["scripts/rustc-suite.ts"]),
  "src/**",
  "std/**",
  "vendor/**",
  "builtins/**",
  "typescript/**",
  "build.rs",
  "Cargo.toml",
  "Cargo.lock",
  "rust-toolchain.toml",
  "package.json",
  "bun.lock",
  "test/rustc-*.txt",
  ".github/workflows/check.yml",
  ".github/workflows/rustc-tests.yml",
  ".github/actions/**",
];

const rustcGlobs = rustcPaths.map((path) => new Bun.Glob(path));

/** Whether a change of `files` touches what rustc's tests run. */
export function touchesRustc(files: string[]): boolean {
  return files.some((file) => rustcGlobs.some((glob) => glob.match(file)));
}

/** The corpus cases among a change's `files`, `test/corpus/<case>.rs`, by
 * name, those it adds or edits: each of their metamorphic rewrites runs,
 * as a case that runs as Rust does may not rewritten, #1115's moved into
 * a library (#1164, #1165). */
export function changedCases(files: string[]): string[] {
  return [...new Set(files.flatMap((file) => /^test\/corpus\/([^/]+\.rs)$/.exec(file)?.[1] ?? []))].sort();
}

/** Whether a change of `files` touches the numbers' code. */
export function touchesNumbers(files: string[]): boolean {
  return files.some((file) => numberGlobs.some((glob) => glob.match(file)));
}

/** What the twin net pits against std's own bodies, as globs of the
 * repository's paths: the iterators' code, the runtime's helpers among it,
 * and the net's list of the helpers it can't pit (#1205). */
export const twinPaths = [...iteratorPaths, "test/twin-net.ts"];

const twinGlobs = twinPaths.map((path) => new Bun.Glob(path));

/** Whether a change of `files` touches what the twin net pits. */
export function touchesTwins(files: string[]): boolean {
  return files.some((file) => twinGlobs.some((glob) => glob.match(file)));
}

/** Whether a change of `files` touches the iterators' code. */
export function touchesIterators(files: string[]): boolean {
  return files.some((file) => globs.some((glob) => glob.match(file)));
}

/** Each run, by its name, of a test file with the settings that select
 * its part: the matrix's slice and the conformance table's, `ITERATOR_TABLE`
 * (`slice()` of each, the test's default); and the corpus with std's
 * iterator structs alone, `ITERATOR_FORMS=corpus`, whose miss let #766
 * reach `main`. */
export const runs = {
  matrix: [{ name: "the matrix's slice", file: "test/iterator-matrix.test.ts", env: { ITERATOR_TABLE: "matrix" } }],
  conformance: [{ name: "the conformance table's slice", file: "test/iterator-matrix.test.ts", env: { ITERATOR_TABLE: "conformance" } }],
  // Each compiled form copied each way and stepped by each ability beside
  // its original, its slice (test/copy-matrix.ts): some 25 s in 10 parts.
  copies: [{ name: "the copy matrix's slice", file: "test/iterator-matrix.test.ts", env: { ITERATOR_TABLE: "copies" } }],
  // Each adapter through each step std's writes, by each path generic and
  // boxed code reaches it (test/iterator-paths.ts, #2112): all of it, 48
  // batches, 20 s cold in 4 parts on the Mac mini.
  paths: [{ name: "the paths table", file: "test/iterator-matrix.test.ts", env: { ITERATOR_TABLE: "paths", ITERATOR_MATRIX: "all" } }],
  forms: [{ name: "the corpus with std's iterator structs", file: "test/iterator-forms.test.ts", env: { ITERATOR_FORMS: "corpus" } }],
  format: [{ name: "the formatting grid's slice", file: "test/iterator-matrix.test.ts", env: { ITERATOR_TABLE: "format" } }],
  numbers: [{ name: "the numbers grid's slice", file: "test/iterator-matrix.test.ts", env: { ITERATOR_TABLE: "numbers" } }],
  // All of it, in one part: some 20 s.
  declarations: [{ name: "the declarations' TypeScript callers", file: "test/declarations-conformance.test.ts", env: {} }],
  // All of it, its slice: some 8 s.
  async: [{ name: "the async table", file: "test/iterator-matrix.test.ts", env: { ITERATOR_TABLE: "async" } }],
  // All of it, its slice: some 20 s in 8 parts.
  consumers: [{ name: "the consumer net", file: "test/iterator-matrix.test.ts", env: { ITERATOR_TABLE: "consumers" } }],
  // All of it, its slice: some 160 cases in 3 programs.
  std: [{ name: "the std net", file: "test/iterator-matrix.test.ts", env: { ITERATOR_TABLE: "std" } }],
  // Each rewrite of each case the change adds or edits, the list `run`
  // writes (#1165).
  metamorphic: [{ name: "the metamorphic rewrites of the change's cases", file: "test/metamorphic.test.ts", env: {} }],
  // The slice's seeds, each a program of unsafe Rust over plain data, Miri
  // where it's installed: some 30 s in 8 parts.
  memory: [{ name: "the memory net's slice", file: "test/memory-net.test.ts", env: {} }],
  // test/tidy-stages.test.ts' intervals family: its few cases, each
  // without MIR's intervals' proofs, against native Rust (#2112 B).
  intervals: [{ name: "the intervals' variant", file: "test/tidy-stages.test.ts", env: { TIDY_VARIANTS: "intervals" } }],
  // Each table against std's own bodies, no native run (#1205): each
  // table's slice, as a push's run of it against native Rust takes, the
  // whole net 9 min on CI's machine; `twinsAll`, all of each but the
  // matrix's sample, the nightly's.
  twins: twinTables({}),
  twinsAll: twinTables({ all: true }),
} as const;

/** The most corpus cases a run of the change's metamorphic rewrites takes,
 * some 6 a part, about 2 min a part on CI's machine. */
export const metamorphicMost = 24;

/** The twin net's runs, each table's against std's own bodies (#1205):
 * its slice, or of `all`, every batch but the matrix's, of which a sample
 * of each adapter with each consumer. */
function twinTables({ all = false }: { all?: boolean }) {
  return (["conformance", "points", "unwind", "consumers", "std", "matrix"] as const).map((table) => ({
    name: `the ${table} table against std's bodies`,
    file: "test/iterator-matrix.test.ts",
    env: {
      ITERATOR_TABLE: table,
      ITERATOR_ORACLE: "std",
      ...(all ? { ITERATOR_MATRIX: table === "matrix" ? "pairs" : "all" } : {}),
    },
  }));
}

/** Where each run's parts write their JUnit reports, each beside what ran
 * it, `<report>.json`, a `SliceReport`: a check's jobs that run them
 * upload them, and a failed check's `attribute` runs each failing test
 * again at the batch's commits, by its run's file and settings (#2003). */
export const sliceReports = join(root, "target", "junit", "slices");

/** What ran a slice's report: the run's name, its file and its settings. */
export type SliceReport = { of: string; name: string; file: string; env: Record<string, string> };

/** Where part `part` of `parts` of a run writes its JUnit report, with
 * what ran it written beside it, `<report>.json`, which a failed check's
 * `attribute` reads to run each failing test again (#2003). */
export function sliceReport(ran: SliceReport, part: number, parts: number, into = sliceReports): string {
  const report = join(into, `${ran.of}-${ran.name.replace(/\W+/g, "-")}-${part}-of-${parts}.xml`);
  mkdirSync(into, { recursive: true });
  writeFileSync(`${report}.json`, JSON.stringify(ran satisfies SliceReport));
  return report;
}

/** `of`, in `parts` processes at once, each its share of the cases
 * (`RUST_JS_CASES`), or of those `list` names, a line each, as plain
 * `check:fast` picks the corpus's by the test map (#876), their logs
 * printed as each ends: whether every part passed, and how long it took. */
export async function run(of: keyof typeof runs | "rows", parts: number, list?: string): Promise<boolean> {
  // The metamorphic net's, of the corpus cases the change adds or edits
  // since `list`, a revision, CI's `since`, or since it left `main`, the
  // working tree's too, a line each in a list of its own: none, no run.
  if (of === "metamorphic") {
    const changed = changedCases(changedSince(list));
    if (changed.length === 0) {
      console.log("the change adds or edits no corpus case: no metamorphic rewrite to run");
      return true;
    }
    // At most `metamorphicMost` of them, those a hash of their name picks,
    // the same each run: a `since` long ago, of checks cancelled or red,
    // named some 100, whose rewrites outran the job (dev-loop). The
    // nightly's runs every case's.
    const cases = changed.length <= metamorphicMost ? changed : [...changed].sort((a, b) => Bun.hash(a).toString().localeCompare(Bun.hash(b).toString())).slice(0, metamorphicMost).sort();
    if (cases.length < changed.length) console.log(`${cases.length} of the ${changed.length} cases the change adds or edits, by a hash of their names; the nightly's run has the rest`);
    list = join(root, "target", "metamorphic-cases.txt");
    mkdirSync(dirname(list), { recursive: true });
    writeFileSync(list, `${cases.join("\n")}\n`);
    console.log(`the metamorphic rewrites of ${cases.join(", ")}`);
    parts = Math.min(parts, cases.length);
  }
  // The twin net's helpers, each batch's, of this run alone (#1205).
  if (of === "twins" || of === "twinsAll") rmSync(twinHelpers, { recursive: true, force: true });
  let passed = true;
  const each: Run[] = of === "rows" ? changeRows(parts) : [...runs[of]];
  for (const { name, file, env, rate } of each) {
    const start = Date.now();
    const parted = Array.from({ length: parts }, async (_, i) => {
      const report = sliceReport({ of, name, file, env }, i + 1, parts);
      const p = Bun.spawn(["bun", "test", "--timeout=600000", "--reporter=junit", `--reporter-outfile=${report}`, file], {
        cwd: root,
        env: { ...process.env, ...env, RUST_JS_CASES: `${i + 1}/${parts}${list === undefined ? "" : `:${list}`}` },
        stdout: "pipe",
        stderr: "pipe",
      });
      // Each line as it comes, its part's name before it, so a run that's
      // killed shows what it had done (dev-loop).
      const said = (stream: ReadableStream<Uint8Array>) => lines(stream, (line) => console.log(`[${name}, part ${i + 1}/${parts}] ${line}`));
      await Promise.all([said(p.stdout), said(p.stderr)]);
      const code = await p.exited;
      console.log(`── ${name}, part ${i + 1}/${parts} ${code === 0 ? "passed" : "failed"} ──`);
      return code === 0;
    });
    const ok = (await Promise.all(parted)).every(Boolean);
    const seconds = (Date.now() - start) / 1000;
    console.log(`${name} ${ok ? "passed" : "failed"} in ${seconds.toFixed(0)} s`);
    // How fast this machine ran it, which sizes the next change's rows to
    // its time (scripts/iteratorRows.ts's `budgetOf`).
    if (rate !== undefined && rate.rows > 0) noteRate({ ...rate, seconds, parts });
    passed &&= ok;
  }
  // Each helper both builds import is one `shared` says why of; and of the
  // whole net, none it lists is pitted now or imported by none.
  if ((of === "twins" || of === "twinsAll") && list === undefined) {
    const problems = sharedProblems(helpersOf(), of === "twinsAll");
    for (const problem of problems) console.log(problem);
    passed &&= problems.length === 0;
  }
  return passed;
}

/** The runs of the rows the change names, the working tree's since it left
 * `main` (scripts/iteratorRows.ts): each table's, said with why they're
 * picked, hunk by hunk. */
type Run = { name: string; file: string; env: Record<string, string>; rate?: { table: "matrix" | "conformance"; rows: number } };

/** The files changed since `since`, a revision, or since the branch left
 * `main`, the working tree's and what's untracked too: none deleted. */
export function changedSince(since?: string): string[] {
  const git = (args: string[]) => Bun.spawnSync(["git", ...args], { cwd: root }).stdout.toString().split("\n").filter(Boolean);
  if (since !== undefined) return git(["diff", "--name-only", "--diff-filter=d", since, "HEAD"]);
  const base = git(["merge-base", "HEAD", "origin/main"])[0];
  if (base === undefined) return [];
  return [...git(["diff", "--name-only", "--diff-filter=d", base]), ...git(["ls-files", "--others", "--exclude-standard"])];
}

/** What of each table the change names, the working tree's since it left
 * `main`, in `parts` (scripts/iteratorRows.ts): the rows `run rows` runs,
 * and check:fast's pool expects that job to take the time of (#1364). */
export function changePicks(parts: number): Picked[] {
  const git = (args: string[]) => Bun.spawnSync(["git", ...args], { cwd: root }).stdout.toString().split("\n").filter(Boolean);
  const base = git(["merge-base", "HEAD", "origin/main"])[0];
  if (base === undefined) return [];
  const files = [...git(["diff", "--name-only", base]), ...git(["ls-files", "--others", "--exclude-standard"])].filter((file) => touchesIterators([file]));
  return pick(base, undefined, files, parts);
}

function changeRows(parts: number): Run[] {
  return changePicks(parts).map((p) => {
    for (const line of p.why) console.log(`  ${line}`);
    console.log(`${p.table}: ${p.rows} rows, ITERATOR_MATRIX='${p.setting}'`);
    return {
      name: `the ${p.table === "matrix" ? "matrix" : "conformance table"}'s ${p.rows} rows the change names`,
      file: "test/iterator-matrix.test.ts",
      env: { ITERATOR_TABLE: p.table, ITERATOR_MATRIX: p.setting },
      rate: { table: p.table as "matrix" | "conformance", rows: p.rows },
    };
  });
}

if (import.meta.main) {
  const [command, of, count, list] = process.argv.slice(2);
  if (command !== "run" || !(of in runs || of === "rows")) throw new Error("usage: bun scripts/iteratorSlice.ts run matrix|conformance|paths|forms|format|numbers|declarations|async|consumers|memory|intervals|twins|twinsAll|rows [parts [cases]] | metamorphic [parts [since]]");
  const parts = count === undefined ? caseParts(machine().cores * 2) : Number(count);
  process.exit((await run(of as keyof typeof runs | "rows", parts, list)) ? 0 : 1);
}
