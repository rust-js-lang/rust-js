// What each of tidy's passes does to a crate's JS (ADR 0659): the crate
// compiled once per pass with `--tidy-until`, and each pass that changed
// the JS shown with its diff, so a change, or a regression, is traced to
// the pass it first appears in.
//
//   bun scripts/tidy-trace.ts <input.rs> [<text the JS has>] [-- <rustc flags>]
//
// Given a text, only whether it's there after each pass is shown, and
// where it first appears and goes.
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const compiler = join(import.meta.dir, "..", "target", "debug", "rust-js");
const args = process.argv.slice(2);
const dash = args.indexOf("--");
const [input, text] = dash < 0 ? args : args.slice(0, dash);
const rustc = dash < 0 ? [] : args.slice(dash + 1);
if (!input) {
  console.error("usage: bun scripts/tidy-trace.ts <input.rs> [<text the JS has>] [-- <rustc flags>]");
  process.exit(2);
}

const passes = run([compiler, "--tidy-passes"]).trim().split("\n");
const dir = mkdtempSync(join(tmpdir(), "tidy-trace-"));
const output = join(dir, "out.js");

function run(command: string[]): string {
  const result = spawnSync(command[0], command.slice(1), { encoding: "utf8" });
  if (result.status !== 0) {
    console.error(result.stderr);
    process.exit(1);
  }
  return result.stdout;
}

// The JS after `pass`, or after every pass where it's none.
function after(pass?: string): string {
  run([compiler, input, "-o", output, ...(pass ? ["--tidy-until", pass] : []), "--", "-Awarnings", ...rustc]);
  return readFileSync(output, "utf8");
}

function diff(before: string, now: string): string {
  const [a, b] = [join(dir, "a.js"), join(dir, "b.js")];
  writeFileSync(a, before);
  writeFileSync(b, now);
  return spawnSync("diff", ["-u", a, b], { encoding: "utf8" }).stdout.split("\n").slice(2).join("\n");
}

let before = after(passes[0]);
console.log(`${passes[0]}: the JS after it${text ? `, ${before.includes(text) ? "with" : "without"} the text` : ""}`);
if (!text) console.log(before);
for (const pass of passes.slice(1)) {
  const now = after(pass);
  if (now !== before) {
    if (!text) console.log(`\n== ${pass}\n${diff(before, now)}`);
    else if (before.includes(text) !== now.includes(text)) {
      console.log(`${pass}: ${now.includes(text) ? "makes" : "takes away"} the text`);
    }
  }
  before = now;
}
rmSync(dir, { recursive: true });
