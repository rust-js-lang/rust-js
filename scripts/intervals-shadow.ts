// The MIR intervals' shadow over the corpus (#1672 change 1 M2): each
// case compiled with `--intervals-shadow`, which logs each check THIR's
// ranges leave out that MIR's intervals don't prove, and each they keep
// that MIR proves (`mir-only`, what M3 leaves out), the lines gathered
// and counted by kind. Each line is a wrong-answer candidate, or a gap of
// the MIR analysis's precision, triaged on #1672.
//
//   bun scripts/intervals-shadow.ts [--out <file>]
import { mkdtempSync, readFileSync, readdirSync, writeFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { directives } from "../test/corpus-cases";
import { atOnce } from "./shard";

const root = join(import.meta.dir, "..");
const compiler = join(root, "target/debug/rust-js");
const corpus = join(root, "test/corpus");
const outAt = process.argv.indexOf("--out");
const out = outAt >= 0 ? process.argv[outAt + 1] : undefined;
const work = mkdtempSync(join(tmpdir(), "intervals-shadow-"));
const cases = readdirSync(corpus).filter((name) => name.endsWith(".rs")).sort();

async function shadowOf(name: string): Promise<string[]> {
  const file = join(corpus, name);
  const want = directives(readFileSync(file, "utf8"));
  if (typeof want === "string" || want.kind === "compile-fail") return [];
  const log = join(work, `${name}.shadow`);
  const proc = Bun.spawn([compiler, "--intervals-shadow", log, file, "-o", join(work, `${name}.js`), "--", `--edition=${want.edition}`], { stdout: "ignore", stderr: "ignore" });
  const timer = setTimeout(() => proc.kill(), 120_000);
  await proc.exited;
  clearTimeout(timer);
  return existsSync(log) ? readFileSync(log, "utf8").split("\n").filter(Boolean).map((line) => `${line}\t${name}`) : [];
}

const lines: string[] = [];
let next = 0;
await Promise.all(
  Array.from({ length: atOnce("test") }, async () => {
    while (next < cases.length) lines.push(...(await shadowOf(cases[next++])));
  }),
);
lines.sort();
const counts = new Map<string, number>();
for (const line of lines) counts.set(line.split("\t")[0], (counts.get(line.split("\t")[0]) ?? 0) + 1);
console.log(`${cases.length} cases, ${lines.length} lines (a kind: THIR leaves out what MIR doesn't prove; mir-only: MIR proves what THIR keeps): ${[...counts].map(([k, n]) => `${k} ${n}`).join(", ")}`);
if (out) writeFileSync(out, lines.join("\n") + "\n");
