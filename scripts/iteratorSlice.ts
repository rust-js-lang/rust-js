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
import { availableParallelism } from "node:os";
import { join } from "node:path";
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
  "src/lower/recognition/iterators.rs",
  "src/lower/recognition/own_steps.rs",
  "src/lower/recognition/slice_steps.rs",
  "src/lower/recognition/owners.rs",
  "src/lower/recognition/specialization.rs",
  "src/lower/recognition/std_items.rs",
  "src/lower/analysis/iterator_steps.rs",
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
} as const;

/** `of`, in `parts` processes at once, each its share of the cases
 * (`RUST_JS_CASES`), their logs printed as each ends: whether every part
 * passed, and how long it took. */
export async function run(of: keyof typeof runs, parts: number): Promise<boolean> {
  let passed = true;
  for (const { name, file, env } of runs[of]) {
    const start = Date.now();
    const parted = Array.from({ length: parts }, async (_, i) => {
      const p = Bun.spawn(["bun", "test", "--timeout=600000", file], {
        cwd: root,
        env: { ...process.env, ...env, RUST_JS_CASES: `${i + 1}/${parts}` },
        stdout: "pipe",
        stderr: "pipe",
      });
      const [out, err] = await Promise.all([new Response(p.stdout).text(), new Response(p.stderr).text()]);
      const code = await p.exited;
      console.log(`── ${name}, part ${i + 1}/${parts} ──\n${out}${err}`);
      return code === 0;
    });
    const ok = (await Promise.all(parted)).every(Boolean);
    console.log(`${name} ${ok ? "passed" : "failed"} in ${((Date.now() - start) / 1000).toFixed(0)} s`);
    passed &&= ok;
  }
  return passed;
}

if (import.meta.main) {
  const [command, of, count] = process.argv.slice(2);
  if (command !== "run" || !(of in runs)) throw new Error("usage: bun scripts/iteratorSlice.ts run matrix|conformance|forms [parts]");
  const parts = count === undefined ? caseParts(availableParallelism() * 2) : Number(count);
  process.exit((await run(of as keyof typeof runs, parts)) ? 0 : 1);
}
