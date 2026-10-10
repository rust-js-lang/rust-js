// The corpus with bodies lowered from their MIR (ADR 0364): how many cases
// run as native Rust does, or are refused as `compile-fail` says, a case
// THIR gets wrong, `ignore-rust-js`, among them where MIR gets it right, which pass that `test/mir-corpus.txt` doesn't
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
// Each case that failed, and what the run said of it before.
const failed = new Map<string, string>();
let said = "";
for (const line of log.split("\n")) {
  const m = /^\(fail\) ([a-z0-9_]+\.rs)/.exec(line);
  if (m) failed.set(m[1], said);
  said = m || line.startsWith("(pass)") ? "" : said + line + "\n";
}
const running = readdirSync(corpus).filter((f) => f.endsWith(".rs"));
const ignored = (f: string) => /^\/\/@ ignore-rust-js/m.test(readFileSync(join(corpus, f), "utf8"));
// One THIR gets wrong passes from MIR where its test says it passes now.
// So does one THIR refuses as a library that MIR builds as one, which
// `mir.test.ts` runs as a program, its only problem that it builds now.
const only = (f: string, problem: string) => {
  const said = failed.get(f) ?? "";
  return said.includes(problem) && [...said.matchAll(/^\+ {3}"/gm)].length === 1;
};
const passesFromMir = (f: string) =>
  ignored(f) ? only(f, "it passes now") : !failed.has(f) || only(f, "it compiles as a library now");
const passing = running.filter(passesFromMir).sort();
// A case passes only if it ran: a compiler that doesn't build runs none.
const passes = Number(/^ (\d+) pass$/m.exec(log)?.[1] ?? 0);
if (passes < passing.filter((f) => !failed.has(f)).length) {
  console.error(`the corpus didn't run: ${passes} passed, ${passing.length} didn't fail\n${log.slice(-2000)}`);
  process.exit(1);
}
const listFile = join(root, "test/mir-corpus.txt");
const listed = new Set(readFileSync(listFile, "utf8").split("\n").filter(Boolean));
const fresh = passing.filter((f) => !listed.has(f));
const lost = [...listed].filter((f) => !passing.includes(f));
console.log(`${passing.length} of ${running.length} cases pass from their MIR`);
if (fresh.length) console.log(`new: ${fresh.join(" ")}`);
if (lost.length) console.log(`REGRESSED: ${lost.join(" ")}`);
const reasons = new Map<string, number>();
for (const m of log.matchAll(/rust-js does not support (.*?) yet/g)) {
  const reason = m[1].replace(/`[^`]*`/g, (s) => (m[1].startsWith("`") ? s : "`X`"));
  reasons.set(reason, (reasons.get(reason) ?? 0) + 1);
}
for (const [reason, n] of [...reasons].sort((a, b) => b[1] - a[1]).slice(0, 40)) console.log(`${String(n).padStart(5)}  ${reason}`);
if (process.argv.includes("--write")) writeFileSync(listFile, [...new Set([...listed, ...passing])].sort().join("\n") + "\n");
