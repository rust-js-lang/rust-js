// The iterator matrix's slice (ADR 1090, #731): the batches of
// test/iterator-matrix.test.ts that a change to the iterators runs, in
// each push's check (check.yml's `iterators`) and in `check:fast`. What
// the iterators' code is, `iteratorPaths`, is said here alone:
// test/architecture-iterators.test.ts holds it to docs/architecture.md's
// owners of iterators.
//
//   bun scripts/iteratorSlice.ts run [parts]   both tables' slices, each in parts at once, against native Rust
import { availableParallelism } from "node:os";
import { join } from "node:path";
import { caseParts } from "./shard";

const root = join(import.meta.dir, "..");

/** The iterators' code, as globs of the repository's paths: their
 * lowering, their recognition and the facts of their steps, the runtime's
 * helpers and its package, std's compiled iterators, and the matrix. */
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
  "scripts/iteratorSlice.ts",
];

const globs = iteratorPaths.map((path) => new Bun.Glob(path));

/** Whether a change of `files` touches the iterators' code. */
export function touchesIterators(files: string[]): boolean {
  return files.some((file) => globs.some((glob) => glob.match(file)));
}

/** The tables whose slices it runs, the matrix's (ADR 1090) and the
 * conformance table's (ADR 1092), each `ITERATOR_TABLE`'s. */
export const tables = ["matrix", "conformance"] as const;

/** Each table's slice in turn, in `parts` processes at once, each its
 * share of the batches (`RUST_JS_CASES`), their logs printed as each ends:
 * whether every part of each passed, and how long each table took. */
export async function runSlice(parts: number): Promise<boolean> {
  let passed = true;
  for (const table of tables) {
    const start = Date.now();
    const runs = Array.from({ length: parts }, async (_, i) => {
      const p = Bun.spawn(["bun", "test", "--timeout=600000", "test/iterator-matrix.test.ts"], {
        cwd: root,
        env: { ...process.env, ITERATOR_TABLE: table, RUST_JS_CASES: `${i + 1}/${parts}` },
        stdout: "pipe",
        stderr: "pipe",
      });
      const [out, err] = await Promise.all([new Response(p.stdout).text(), new Response(p.stderr).text()]);
      const code = await p.exited;
      console.log(`── the ${table}'s slice, part ${i + 1}/${parts} ──\n${out}${err}`);
      return code === 0;
    });
    const ok = (await Promise.all(runs)).every(Boolean);
    console.log(`the ${table}'s slice ${ok ? "passed" : "failed"} in ${((Date.now() - start) / 1000).toFixed(0)} s`);
    passed &&= ok;
  }
  return passed;
}

if (import.meta.main) {
  const [command, count] = process.argv.slice(2);
  if (command !== "run") throw new Error("usage: bun scripts/iteratorSlice.ts run [parts]");
  const parts = count === undefined ? caseParts(availableParallelism() * 2) : Number(count);
  process.exit((await runSlice(parts)) ? 0 : 1);
}
