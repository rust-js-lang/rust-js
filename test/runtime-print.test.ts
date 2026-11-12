// The runtime's printing (ADRs 0087, 1130): stdout's line, as a line is
// formatted through it, `$printing` and `$out`, run as the compiler prints
// its helpers, so a mutation of them is the one run.

import { beforeAll, expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { node } from "./programs";
import { buildCompiler, compiler, fixture, run } from "./support";

beforeAll(buildCompiler, 600_000);

// Stdout's line, as a line is formatted through it (#1490, ADR 1130), is
// written at each newline, as std's `LineWriter` writes it: standard error
// written after a newline comes after the line it ends, and before the
// next one's text, which is held till its own newline. Each stream's
// writes are recorded in one list, as a terminal shows both.
test("stdout's line is written at its newline, standard error at once", () => {
  const module = join(fixture("runtime-line-writer"), "runtime.js");
  writeFileSync(module, run([compiler, "--runtime-module"]));
  const script = `
const { $out, $printing } = await import(${JSON.stringify(module)});
const seen = [];
const shown = process.stdout.write.bind(process.stdout);
process.stdout.write = (text) => (seen.push("out " + JSON.stringify(String(text))), true);
process.stderr.write = (text) => (seen.push("err " + JSON.stringify(String(text))), true);
$printing(() => {
  $out("a ");
  $out("inner\\n");
  console.error("err");
  $out("b");
  $out("\\n");
});
$printing(() => $out("open "));
shown(seen.join("|") + "\\n");
`;
  expect(run([node ?? "node", "--input-type=module", "--eval", script])).toBe(
    'out "a inner\\n"|err "err\\n"|out "b\\n"|out "open "\n',
  );
});
