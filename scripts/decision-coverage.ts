// Decision coverage (docs/finding-wrong-answers.md): which answers of the
// compiler's decisions the nets' programs reach, and which none does. Each
// net runs with `RUST_JS_DECISIONS` naming a file of its own, which each
// compile appends its counts to (`src/decisions.rs`), every decision's
// every answer, `0` of one it never gave, tagged by its build and its
// shape; this merges them: the pairs reached, those never reached, those
// reached in one build or one shape alone, and those only one net reaches.
// A never-reached answer is where a wrong answer hides unseen: each is a
// generator template or a corpus case to write.
//
//   bun scripts/decision-coverage.ts [net ...]       the nets, or those named
//   bun scripts/decision-coverage.ts --merge [dir]   the last run's files again, or dir's
//   bun scripts/decision-coverage.ts --sum file      file's lines summed, in place
//
// Its report is target/decision-coverage/report.md. The compile cache is
// off for it, as a compile it gives again counts nothing. The nightly
// counts as it runs each net (nightly.yml's `RUST_JS_DECISIONS`): each job
// sums its file, a line a compile, and its `decision-coverage` job merges
// them, a file a job, the net its name before its first dot.
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { availableParallelism } from "node:os";
import { join } from "node:path";

const root = join(import.meta.dir, "..");
const out = join(root, "target", "decision-coverage");
const counts = join(out, "counts");
const compiler = join(root, "target", "debug", "rust-js");

/** Each net, as the nightly runs all of it: its runs, each a test file,
 * the settings that pick all its cases (a net's test file alone runs a
 * push's sample), and how many parts it runs in at once. The iterator
 * matrix is its sample of each adapter with each consumer, `pairs`, as
 * all of it is 24 parts of CI's 6 machines. */
type Run = { file: string; env?: Record<string, string>; parts?: number };
const matrix = (table: string, setting = "all", extra: Record<string, string> = {}): Run => ({
  file: "test/iterator-matrix.test.ts",
  env: { ITERATOR_TABLE: table, ITERATOR_MATRIX: setting, ...extra },
  parts: 4,
});
const tables = ["conformance", "points", "unwind", "format", "numbers", "async", "consumers", "std"];
const nets: Record<string, Run[]> = {
  corpus: [{ file: "test/corpus.test.ts", parts: 8 }],
  "iterator-forms": [{ file: "test/iterator-forms.test.ts", env: { ITERATOR_FORMS: "corpus" }, parts: 6 }],
  // Generated programs split by their seeds, as the file runs its seeds one
  // after another: 240 of them.
  fuzz: [1, 61, 121, 181].map((start) => ({ file: "test/fuzz.test.ts", env: { FUZZ_START: `${start}`, FUZZ_SEEDS: "60" } })),
  memory: [{ file: "test/memory-net.test.ts", env: { MEMORY_NET: "all" }, parts: 4 }],
  iterators: [matrix("matrix", "pairs"), ...tables.map((t) => matrix(t))],
  twin: [...tables.filter((t) => ["conformance", "points", "unwind", "consumers", "std"].includes(t)), "matrix"].map((t) =>
    matrix(t, t === "matrix" ? "pairs" : "all", { ITERATOR_ORACLE: "std" }),
  ),
  tables: [
    { file: "test/mut-ref-matrix.test.ts", env: { MUT_REF_MATRIX: "all" }, parts: 2 },
    { file: "test/none-matrix.test.ts", env: { NONE_MATRIX: "all" }, parts: 2 },
  ],
  metamorphic: [{ file: "test/metamorphic.test.ts", env: { METAMORPHIC: "all" }, parts: 8 }],
  neighbours: [{ file: "test/neighbours.test.ts", env: { NEIGHBOURS: "all" }, parts: 4 }],
  "object-model": [{ file: "test/object-model.test.ts", env: { OBJECT_MODEL: "all" }, parts: 4 }],
  crates: [{ file: "test/crates.test.ts" }, { file: "test/crates-compiled-here.test.ts" }],
};

type Job = { name: string; files: string[]; env: Record<string, string> };

function jobs(names: string[]): Job[] {
  return names.flatMap((name) => {
    const runs = nets[name];
    if (!runs) throw new Error(`no net named ${name}: ${Object.keys(nets).join(", ")}`);
    return runs.flatMap((run, r) => {
      const parts = run.parts ?? 1;
      return Array.from({ length: parts }, (_, i) => {
        const tag = `${name}.${r + 1}-${i + 1}`;
        const env: Record<string, string> = { ...run.env, RUST_JS_DECISIONS: join(counts, `${tag}.tsv`) };
        if (parts > 1) env.RUST_JS_CASES = `${i + 1}/${parts}`;
        return { name: tag, files: [run.file], env };
      });
    });
  });
}

async function run(job: Job): Promise<{ name: string; code: number; seconds: number }> {
  const started = performance.now();
  const proc = Bun.spawn(["bun", "test", "--timeout=900000", ...job.files], {
    cwd: root,
    env: { ...process.env, ...job.env, RUST_JS_COMPILER: compiler, RUST_JS_COMPILE_CACHE_MB: "0" },
    stdout: "ignore",
    stderr: "ignore",
  });
  // Each net past its time is a hang, not slow coverage (AGENTS.md).
  const timer = setTimeout(() => proc.kill(), 90 * 60 * 1000);
  const code = await proc.exited;
  clearTimeout(timer);
  return { name: job.name, code, seconds: Math.round((performance.now() - started) / 1000) };
}

async function pool(all: Job[], width: number) {
  const results: { name: string; code: number; seconds: number }[] = [];
  const queue = [...all];
  await Promise.all(
    Array.from({ length: width }, async () => {
      for (let job = queue.shift(); job; job = queue.shift()) {
        const result = await run(job);
        console.log(`${result.name}: exit ${result.code}, ${result.seconds} s`);
        results.push(result);
      }
    }),
  );
  return results;
}

type Pair = { decision: string; answer: string; owner: string; count: number; nets: Set<string>; builds: Set<string>; shapes: Set<string> };

/** The net a counts file is of: its name before its first dot. */
const netOf = (file: string) => file.split(".")[0];

/** `file`'s lines, a compile's every decision's every answer each, summed
 * by decision, answer, build and shape: what the report reads of them, a
 * file of some 50 lines where a net's was one of 49 a compile. */
function sum(file: string) {
  const sums = new Map<string, number>();
  for (const line of readFileSync(file, "utf8").split("\n")) {
    const [decision, answer, n, build, shape, owner] = line.split("\t");
    if (!owner) continue;
    const key = [decision, answer, build, shape, owner].join("\t");
    sums.set(key, (sums.get(key) ?? 0) + Number(n));
  }
  const lines = [...sums].map(([key, n]) => {
    const [decision, answer, build, shape, owner] = key.split("\t");
    return `${decision}\t${answer}\t${n}\t${build}\t${shape}\t${owner}\n`;
  });
  writeFileSync(file, lines.join(""));
}

function merge(dir: string): Map<string, Pair> {
  const pairs = new Map<string, Pair>();
  // What each net's compiles were of: a build or shape none of them was
  // can't be said to be missed.
  for (const file of readdirSync(dir).filter((f) => f.endsWith(".tsv"))) {
    const net = netOf(file);
    for (const line of readFileSync(join(dir, file), "utf8").split("\n")) {
      const [decision, answer, n, build, shape, owner] = line.split("\t");
      if (!owner) continue;
      const key = `${decision}\t${answer}`;
      const pair = pairs.get(key) ?? { decision, answer, owner, count: 0, nets: new Set(), builds: new Set(), shapes: new Set() };
      pairs.set(key, pair);
      const count = Number(n);
      if (count > 0) {
        pair.count += count;
        pair.nets.add(net);
        pair.builds.add(build);
        pair.shapes.add(shape);
      }
    }
  }
  return pairs;
}

function report(pairs: Map<string, Pair>, runs: { name: string; code: number; seconds: number }[]): string {
  const all = [...pairs.values()];
  const reached = all.filter((p) => p.count > 0);
  const never = all.filter((p) => p.count === 0);
  const oneBuild = reached.filter((p) => p.builds.size === 1);
  const oneShape = reached.filter((p) => p.shapes.size === 1);
  const oneNet = reached.filter((p) => p.nets.size === 1);
  const row = (p: Pair) =>
    `| ${p.decision} | ${p.answer} | ${p.count} | ${[...p.nets].sort().join(", ")} | ${[...p.builds].sort().join(", ")} | ${[...p.shapes].sort().join(", ")} | ${p.owner} |`;
  const table = (ps: Pair[]) =>
    ["| Decision | Answer | Count | Nets | Builds | Shapes | Owner |", "|---|---|---|---|---|---|---|", ...ps.map(row)].join("\n");
  const decisions = new Set(all.map((p) => p.decision)).size;
  return [
    "# Decision coverage",
    "",
    `${decisions} decisions, ${all.length} answers: ${reached.length} reached, ${never.length} never.`,
    "",
    "## Never reached",
    "",
    never.length ? table(never) : "None.",
    "",
    "## Reached in one build alone",
    "",
    oneBuild.length ? table(oneBuild) : "None.",
    "",
    "## Reached in one shape alone",
    "",
    oneShape.length ? table(oneShape) : "None.",
    "",
    "## Reached by one net alone",
    "",
    oneNet.length ? table(oneNet) : "None.",
    "",
    "## Every answer",
    "",
    table(all),
    "",
    "## Runs",
    "",
    runs.length ? runs.map((r) => `- ${r.name}: exit ${r.code}, ${r.seconds} s`).join("\n") : "Merged from the last run's files.",
    "",
  ].join("\n");
}

const args = process.argv.slice(2);
if (args[0] === "--sum") {
  if (existsSync(args[1])) sum(args[1]);
  process.exit(0);
}
let runs: { name: string; code: number; seconds: number }[] = [];
const merging = args[0] === "--merge";
const dir = merging && args[1] ? args[1] : counts;
if (!merging) {
  const build = Bun.spawnSync(["cargo", "build", "--quiet"], { cwd: root, stdout: "inherit", stderr: "inherit" });
  if (build.exitCode !== 0) process.exit(build.exitCode ?? 1);
  const names = args.length ? args : Object.keys(nets);
  // A run of the nets named keeps the others' counts of the last run.
  mkdirSync(counts, { recursive: true });
  for (const file of readdirSync(counts)) if (names.includes(netOf(file))) rmSync(join(counts, file));
  // Each net's tests run compiles of their own at once; half the cores'
  // nets at a time keeps every core busy without starving them.
  runs = await pool(jobs(names), Math.max(2, Math.floor(availableParallelism() / 2)));
}
if (!existsSync(dir)) throw new Error(`no counts in ${dir}: run it without --merge first`);
const text = report(merge(dir), runs);
mkdirSync(out, { recursive: true });
writeFileSync(join(out, "report.md"), text);
console.log(text.split("\n## Reached in one build alone")[0]);
console.log(`\nThe whole report: ${join(out, "report.md")}`);
