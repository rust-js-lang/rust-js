// The corpus with bodies lowered from their MIR (ADR 0364): how many cases
// run as native Rust does, which pass that `test/mir-corpus.txt` doesn't
// list yet, and what MIR lowering doesn't support yet, most often first.
//
//   bun scripts/mirCorpus.ts           report
//   bun scripts/mirCorpus.ts --write   and add what passes now to the list

import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const root = join(import.meta.dir, "..");
const corpus = join(root, "test/corpus");
const run = Bun.spawnSync(["bun", "test", "test/corpus.test.ts"], {
  cwd: root,
  env: { ...process.env, RUST_JS_MIR: "1", RUST_JS_SNAPSHOTS: "ignore" },
  stdout: "pipe",
  stderr: "pipe",
});
const log = run.stdout.toString() + run.stderr.toString();
const failed = new Set([...log.matchAll(/\(fail\) ([a-z0-9_]+\.rs)/g)].map((m) => m[1]));
const running = readdirSync(corpus)
  .filter((f) => f.endsWith(".rs"))
  .filter((f) => !/^\/\/@ (compile-fail|ignore-rust-js)/m.test(readFileSync(join(corpus, f), "utf8")));
const passing = running.filter((f) => !failed.has(f)).sort();
// A case passes only if it ran: a compiler that doesn't build runs none.
const passes = Number(/^ (\d+) pass$/m.exec(log)?.[1] ?? 0);
if (passes < passing.length) {
  console.error(`the corpus didn't run: ${passes} passed, ${passing.length} didn't fail\n${log.slice(-2000)}`);
  process.exit(1);
}
const listFile = join(root, "test/mir-corpus.txt");
const listed = new Set(readFileSync(listFile, "utf8").split("\n").filter(Boolean));
const fresh = passing.filter((f) => !listed.has(f));
const lost = [...listed].filter((f) => !passing.includes(f));
console.log(`${passing.length} of ${running.length} running cases pass from their MIR`);
if (fresh.length) console.log(`new: ${fresh.join(" ")}`);
if (lost.length) console.log(`REGRESSED: ${lost.join(" ")}`);
const reasons = new Map<string, number>();
for (const m of log.matchAll(/rust-js does not support (.*?) yet/g)) {
  const reason = m[1].replace(/`[^`]*`/g, (s) => (m[1].startsWith("`") ? s : "`X`"));
  reasons.set(reason, (reasons.get(reason) ?? 0) + 1);
}
for (const [reason, n] of [...reasons].sort((a, b) => b[1] - a[1]).slice(0, 40)) console.log(`${String(n).padStart(5)}  ${reason}`);
if (process.argv.includes("--write")) writeFileSync(listFile, [...new Set([...listed, ...passing])].sort().join("\n") + "\n");
