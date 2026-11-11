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
//   bun scripts/decision-coverage.ts [net ...]   the nets, or those named
//   bun scripts/decision-coverage.ts --merge     the last run's files again
//
// Its report is target/decision-coverage/report.md. The compile cache is
// off for it, as a compile it gives again counts nothing.
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { availableParallelism } from "node:os";
import { join } from "node:path";

const root = join(import.meta.dir, "..");
const out = join(root, "target", "decision-coverage");
const counts = join(out, "counts");
const compiler = join(root, "target", "debug", "rust-js");

/** Each net: its test files, and how many parts it runs in at once. */
const nets: Record<string, { files: string[]; parts?: number }> = {
  corpus: { files: ["test/corpus.test.ts"], parts: 8 },
  "iterator-forms": { files: ["test/iterator-forms.test.ts"] },
  fuzz: { files: ["test/fuzz.test.ts"] },
  memory: { files: ["test/memory-net.test.ts"] },
  consumer: { files: ["test/consumer-net.test.ts"] },
  std: { files: ["test/std-net.test.ts"] },
  twin: { files: ["test/twin-net.test.ts"] },
  tables: {
    files: ["test/matrix.test.ts", "test/iterator-matrix.test.ts", "test/mut-ref-matrix.test.ts", "test/none-matrix.test.ts"],
  },
  metamorphic: { files: ["test/metamorphic.test.ts"] },
  neighbours: { files: ["test/neighbours.test.ts"] },
  crates: { files: ["test/crates.test.ts", "test/crates-compiled-here.test.ts"] },
};

type Job = { name: string; file: string; env: Record<string, string> };

function jobs(names: string[]): Job[] {
  return names.flatMap((name) => {
    const net = nets[name];
    if (!net) throw new Error(`no net named ${name}: ${Object.keys(nets).join(", ")}`);
    const parts = net.parts ?? 1;
    return Array.from({ length: parts }, (_, i) => {
      const tag = parts > 1 ? `${name}.${i + 1}` : name;
      const env: Record<string, string> = { RUST_JS_DECISIONS: join(counts, `${tag}.tsv`) };
      if (parts > 1) env.RUST_JS_CASES = `${i + 1}/${parts}`;
      return { name: tag, file: net.files.join(" "), env };
    });
  });
}

async function run(job: Job): Promise<{ name: string; code: number; seconds: number }> {
  const started = performance.now();
  const files = job.file.split(" ");
  const proc = Bun.spawn(["bun", "test", "--timeout=600000", ...files], {
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

function merge(): Map<string, Pair> {
  const pairs = new Map<string, Pair>();
  // What each net's compiles were of: a build or shape none of them was
  // can't be said to be missed.
  for (const file of readdirSync(counts).filter((f) => f.endsWith(".tsv"))) {
    const net = file.replace(/(\.\d+)?\.tsv$/, "");
    for (const line of readFileSync(join(counts, file), "utf8").split("\n")) {
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
let runs: { name: string; code: number; seconds: number }[] = [];
if (!args.includes("--merge")) {
  const build = Bun.spawnSync(["cargo", "build", "--quiet"], { cwd: root, stdout: "inherit", stderr: "inherit" });
  if (build.exitCode !== 0) process.exit(build.exitCode ?? 1);
  rmSync(counts, { recursive: true, force: true });
  mkdirSync(counts, { recursive: true });
  const names = args.length ? args : Object.keys(nets);
  // Each net's tests run compiles of their own at once; half the cores'
  // nets at a time keeps every core busy without starving them.
  runs = await pool(jobs(names), Math.max(2, Math.floor(availableParallelism() / 2)));
}
if (!existsSync(counts)) throw new Error("no counts: run it without --merge first");
const text = report(merge(), runs);
writeFileSync(join(out, "report.md"), text);
console.log(text.split("\n## Reached in one build alone")[0]);
console.log(`\nThe whole report: ${join(out, "report.md")}`);
