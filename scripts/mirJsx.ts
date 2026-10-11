// The JSX tests with bodies lowered from their MIR (ADR 0364): how many pass,
// which pass that `test/mir-jsx.txt` doesn't list yet, and which it lists
// that fail.
//
//   bun scripts/mirJsx.ts           report
//   bun scripts/mirJsx.ts --write   and add what passes now to the list

import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const root = join(import.meta.dir, "..");
const report = join(mkdtempSync(join(tmpdir(), "mir-jsx-")), "report.xml");
const run = Bun.spawnSync(["bun", "test", "test/jsx.test.ts", "--reporter=junit", `--reporter-outfile=${report}`], {
  cwd: root,
  env: { ...process.env, RUST_JS_MIR: "1", RUST_JS_MIR_ALL: "1" },
  stdout: "pipe",
  stderr: "pipe",
});
const log = run.stdout.toString() + run.stderr.toString();
// Each test the report has, and whether it failed.
const unescape = (text: string) =>
  text.replace(/&(lt|gt|quot|apos|amp|#(\d+));/g, (_, name, code) =>
    code ? String.fromCharCode(Number(code)) : { lt: "<", gt: ">", quot: '"', apos: "'", amp: "&" }[name as string]!,
  );
const cases = [...readFileSync(report, "utf8").matchAll(/<testcase name="([^"]*)"[^>]*?(\/>|>([\s\S]*?)<\/testcase>)/g)].map(
  (m) => ({ name: unescape(m[1]), failed: (m[3] ?? "").includes("<failure") || (m[3] ?? "").includes("<skipped") }),
);
const passing = cases.filter((c) => !c.failed).map((c) => c.name);
const failing = cases.filter((c) => c.failed).map((c) => c.name);
if (passing.length === 0) {
  console.error(`the JSX tests didn't run\n${log.slice(-2000)}`);
  process.exit(1);
}
const listFile = join(root, "test/mir-jsx.txt");
const listed = new Set(readFileSync(listFile, "utf8").split("\n").filter(Boolean));
const fresh = passing.filter((name) => !listed.has(name));
const lost = [...listed].filter((name) => !passing.includes(name));
console.log(`${passing.length} of ${passing.length + failing.length} JSX tests pass from their MIR`);
if (fresh.length) console.log(`new:\n  ${fresh.join("\n  ")}`);
if (lost.length) console.log(`REGRESSED:\n  ${lost.join("\n  ")}`);
const reasons = new Map<string, number>();
for (const m of log.matchAll(/rust-js does not support (.*?) yet/g)) {
  const reason = m[1].replace(/`[^`]*`/g, "`X`");
  reasons.set(reason, (reasons.get(reason) ?? 0) + 1);
}
for (const [reason, n] of [...reasons].sort((a, b) => b[1] - a[1]).slice(0, 40)) console.log(`${String(n).padStart(5)}  ${reason}`);
if (process.argv.includes("--write")) writeFileSync(listFile, [...new Set([...listed, ...passing])].sort().join("\n") + "\n");
