// What a change to the iterators runs, in each push's check (check.yml's
// `iterators`, `iterator-conformance` and `iterator-forms`, at once) and in
// `check:fast` (#731, #766): the slices of the iterator matrix (ADR 1090)
// and of the conformance table (ADR 1092), and every corpus case built
// with std's iterator structs alone (ADR 0908), the user's choice of
// 2026-11-05. What the iterators' code is, `iteratorPaths`, is said
// here alone: test/architecture-iterators.test.ts holds it to
// docs/architecture.md's owners of iterators.
//
//   bun scripts/iteratorSlice.ts run matrix [parts]        the matrix's slice, in parts at once, against native Rust
//   bun scripts/iteratorSlice.ts run conformance [parts]   the conformance table's
//   bun scripts/iteratorSlice.ts run forms [parts]         the corpus with std's iterator structs
//   bun scripts/iteratorSlice.ts run rows [parts]          the rows of both tables the change names (scripts/iteratorRows.ts, #818)
//   bun scripts/iteratorSlice.ts run format [parts]        the formatting grid's slice (#941), of a change to the formatting code
//   bun scripts/iteratorSlice.ts run numbers [parts]       the numbers grid's slice (#940), of a change to the numbers' code
//   bun scripts/iteratorSlice.ts run declarations 1        the declarations' TypeScript callers (#822), of a change to what a `None` is at the JS boundary
//   bun scripts/iteratorSlice.ts run async [parts]         the async table (#942), of a change to how a future awaits and drops
//
// And a change to the formatting code, `formatPaths`, runs the formatting
// grid's slice (test/format-grid.ts, #941), each push's check's
// `format-grid` and `check:fast`'s; and to the numbers' code,
// `numberPaths`, the numbers grid's (test/number-grid.ts, #940),
// `number-grid`; and to what a `None` is where JS gives or takes it,
// `declarationPaths`, every TypeScript caller of the declarations
// (test/declarations-conformance.test.ts, #822), `declarations`; and to
// how a future awaits and what it drops as it unwinds, `asyncPaths`, the
// async table, all of it (test/async-points.ts, #942), `async-points`.
import { availableParallelism } from "node:os";
import { join } from "node:path";
import { noteRate, pick } from "./iteratorRows";
import { caseParts } from "./shard";

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
  "src/lower/mir/known_capacity.rs",
  "src/lower/recognition/iterators.rs",
  "src/lower/recognition/own_steps.rs",
  "src/lower/recognition/slice_steps.rs",
  "src/lower/recognition/owners.rs",
  "src/lower/recognition/specialization.rs",
  "src/lower/recognition/std_items.rs",
  "src/lower/analysis/iterator_steps.rs",
  "src/lower/analysis/back_stepped.rs",
  "src/lower/body_queries.rs",
  "src/runtime.rs",
  "src/runtime/**",
  "runtime/index.js",
  "std/iter.js",
  "test/iterator-matrix.ts",
  "test/iterator-matrix.test.ts",
  "test/iterator-matrix-known.txt",
  "test/iterator-forms.test.ts",
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
 * and the presence test JS reads it by; and the callers that test them. */
export const declarationPaths = [
  "src/typescript.rs",
  "src/lower/representation/nones.rs",
  "src/lower/representation/options.rs",
  "src/lower/bindings/nullable.rs",
  "src/js/make.rs",
  "src/js/stmt_walk.rs",
  "test/declarations-conformance.test.ts",
];

const declarationGlobs = declarationPaths.map((path) => new Bun.Glob(path));

/** Whether a change of `files` touches what a `None` is at the JS boundary. */
export function touchesDeclarations(files: string[]): boolean {
  return files.some((file) => declarationGlobs.some((glob) => glob.match(file)));
}

/** How a future awaits and what an async body drops as it unwinds, as
 * globs of the repository's paths: `.await`'s lowering, the drops and
 * their unwinding's owners, and the table and its runner. */
export const asyncPaths = [
  "src/lower/mir/awaits.rs",
  "src/lower/drops/**",
  "src/lower/analysis/unwinding.rs",
  "test/async-points.ts",
  "test/panic-points.ts",
];

const asyncGlobs = asyncPaths.map((path) => new Bun.Glob(path));

/** Whether a change of `files` touches how a future awaits and drops. */
export function touchesAsync(files: string[]): boolean {
  return files.some((file) => asyncGlobs.some((glob) => glob.match(file)));
}

/** Whether a change of `files` touches the numbers' code. */
export function touchesNumbers(files: string[]): boolean {
  return files.some((file) => numberGlobs.some((glob) => glob.match(file)));
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
  forms: [{ name: "the corpus with std's iterator structs", file: "test/iterator-forms.test.ts", env: { ITERATOR_FORMS: "corpus" } }],
  format: [{ name: "the formatting grid's slice", file: "test/iterator-matrix.test.ts", env: { ITERATOR_TABLE: "format" } }],
  numbers: [{ name: "the numbers grid's slice", file: "test/iterator-matrix.test.ts", env: { ITERATOR_TABLE: "numbers" } }],
  // All of it, in one part: some 20 s.
  declarations: [{ name: "the declarations' TypeScript callers", file: "test/declarations-conformance.test.ts", env: {} }],
  // All of it, its slice: some 8 s.
  async: [{ name: "the async table", file: "test/iterator-matrix.test.ts", env: { ITERATOR_TABLE: "async" } }],
} as const;

/** `of`, in `parts` processes at once, each its share of the cases
 * (`RUST_JS_CASES`), or of those `list` names, a line each, as plain
 * `check:fast` picks the corpus's by the test map (#876), their logs
 * printed as each ends: whether every part passed, and how long it took. */
export async function run(of: keyof typeof runs | "rows", parts: number, list?: string): Promise<boolean> {
  let passed = true;
  const each: Run[] = of === "rows" ? changeRows(parts) : [...runs[of]];
  for (const { name, file, env, rate } of each) {
    const start = Date.now();
    const parted = Array.from({ length: parts }, async (_, i) => {
      const p = Bun.spawn(["bun", "test", "--timeout=600000", file], {
        cwd: root,
        env: { ...process.env, ...env, RUST_JS_CASES: `${i + 1}/${parts}${list === undefined ? "" : `:${list}`}` },
        stdout: "pipe",
        stderr: "pipe",
      });
      const [out, err] = await Promise.all([new Response(p.stdout).text(), new Response(p.stderr).text()]);
      const code = await p.exited;
      console.log(`── ${name}, part ${i + 1}/${parts} ──\n${out}${err}`);
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
  return passed;
}

/** The runs of the rows the change names, the working tree's since it left
 * `main` (scripts/iteratorRows.ts): each table's, said with why they're
 * picked, hunk by hunk. */
type Run = { name: string; file: string; env: Record<string, string>; rate?: { table: "matrix" | "conformance"; rows: number } };

function changeRows(parts: number): Run[] {
  const git = (args: string[]) => Bun.spawnSync(["git", ...args], { cwd: root }).stdout.toString().split("\n").filter(Boolean);
  const base = git(["merge-base", "HEAD", "origin/main"])[0];
  if (base === undefined) return [];
  const files = [...git(["diff", "--name-only", base]), ...git(["ls-files", "--others", "--exclude-standard"])].filter((file) => touchesIterators([file]));
  return pick(base, undefined, files, parts).map((p) => {
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
  if (command !== "run" || !(of in runs || of === "rows")) throw new Error("usage: bun scripts/iteratorSlice.ts run matrix|conformance|forms|format|numbers|declarations|async|rows [parts [cases]]");
  const parts = count === undefined ? caseParts(availableParallelism() * 2) : Number(count);
  process.exit((await run(of as keyof typeof runs | "rows", parts, list)) ? 0 : 1);
}
