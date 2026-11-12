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

// What a writer prints as it formats inside a std value's `Debug` is
// marked in its text and put back as the text reaches the line (#1521). A
// mark is no lone surrogate and number alone: a JS string given through
// interop may hold those, `\uDC000\uDC01`, and reaches the line as it is,
// beside a print's mark that's put back.
test("a JS string's lone surrogates reach the line as they are, beside a print's mark", () => {
  const module = join(fixture("runtime-line-marks"), "runtime.js");
  writeFileSync(module, run([compiler, "--runtime-module"]));
  const script = `
const { $out, $printing, $sink } = await import(${JSON.stringify(module)});
const seen = [];
const shown = process.stdout.write.bind(process.stdout);
process.stdout.write = (text) => (seen.push(String(text)), true);
$printing(() => {
  $out("[");
  const f = $sink();
  $out("!");
  f.write("a\\uDC000\\uDC01b\\uDC00");
  $out(f.done());
  $out("]\\n");
});
shown(JSON.stringify(seen.join("")) + "\\n");
`;
  expect(run([node ?? "node", "--input-type=module", "--eval", script])).toBe('"[!a\\udc000\\udc01b\\udc00]\\n"\n');
});
