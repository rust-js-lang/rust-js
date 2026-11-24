// The MIR intervals' shadow over the corpus (#1672): each case compiled
// with `--shadow intervals=<file>`, which logs each check MIR's intervals prove,
// `proved-<kind>` (THIR's ranges, which it once compared them with, gone in
// M4), the lines gathered and counted by kind.
//
//   bun scripts/intervals-shadow.ts [--out <file>]
import { mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { directives } from "../test/corpus-cases";
import { compiler, shadowLines } from "../test/support";
import { atOnce } from "./shard";

const root = join(import.meta.dir, "..");
const corpus = join(root, "test/corpus");
const outAt = process.argv.indexOf("--out");
const out = outAt >= 0 ? process.argv[outAt + 1] : undefined;
const work = mkdtempSync(join(tmpdir(), "intervals-shadow-"));
const cases = readdirSync(corpus).filter((name) => name.endsWith(".rs")).sort();

// The cases whose compile failed: none, or a crash of the analysis would
// pass as a case with nothing to say.
const failed: string[] = [];

async function shadowOf(name: string): Promise<string[]> {
  const file = join(corpus, name);
  const want = directives(readFileSync(file, "utf8"));
  if (typeof want === "string" || want.kind === "compile-fail") return [];
  const log = join(work, `${name}.shadow`);
  const proc = Bun.spawn([compiler, "--shadow", `intervals=${log}`, file, "-o", join(work, `${name}.js`), "--", `--edition=${want.edition}`], { stdout: "ignore", stderr: "pipe" });
  const timer = setTimeout(() => proc.kill(), 120_000);
  const stderr = await new Response(proc.stderr).text();
  const code = await proc.exited;
  clearTimeout(timer);
  // A refusal is the case's to make (`error:`), not a crash.
  if (code !== 0 && (want.kind !== "ignore-rust-js" || stderr.includes("panicked"))) failed.push(name);
  return existsSync(log) ? shadowLines(log, "intervals").map((line) => `${line}\t${name}`) : [];
}

const lines: string[] = [];
let next = 0;
await Promise.all(
  Array.from({ length: atOnce("test") }, async () => {
    while (next < cases.length) lines.push(...(await shadowOf(cases[next++])));
  }),
);
// Each case's JS and log, some 25 MB a run, gone: kept, 261 runs' held 7 GB.
rmSync(work, { recursive: true, force: true });
lines.sort();
const counts = new Map<string, number>();
// Each a site but each case's count of its walks (#2313).
for (const line of lines.filter((l) => !l.startsWith("walked-for-callers\t"))) counts.set(line.split("\t")[0], (counts.get(line.split("\t")[0]) ?? 0) + 1);
console.log(`${cases.length} cases, ${lines.length} sites MIR proves: ${[...counts].map(([k, n]) => `${k} ${n}`).join(", ")}`);
if (out) writeFileSync(out, lines.join("\n") + "\n");
if (failed.length > 0) {
  console.error(`${failed.length} cases failed to compile: ${failed.join(", ")}`);
  process.exit(1);
}
