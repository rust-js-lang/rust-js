// The runtime's printing (ADRs 0087, 1130): stdout's line, as a line is
// formatted through it, `$printing` and `$out`, run as the compiler prints
// its helpers, so a mutation of them is the one run.

import { beforeAll, expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { compileJs, node, runJs, runNative, runtimes } from "./programs";
import { buildCompiler, compiler, fixture, run } from "./support";
import { underLoad } from "../scripts/timing";

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

// The guide's difference 57, the user's decision (#1496, ADR 0262): a
// writer that panics part-way through a `println!`'s line, in a program
// that catches no panic, loses what that line had written, `x half`,
// which native Rust's stdout writes as the program ends. Where a
// `catch_unwind` may catch it, it's kept (corpus case
// panic_part_way_through_a_line_kept_where_caught).
test("a line a writer's panic cuts short is lost where no panic is caught: difference 57", () => {
  const dir = fixture("runtime-line-cut-short");
  const file = join(dir, "case.rs");
  writeFileSync(
    file,
    `use std::fmt;
struct Halfway;
impl fmt::Display for Halfway {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "half")?;
        panic!("in the middle");
    }
}
fn main() {
    println!("x {} y", Halfway);
}
`,
  );
  const native = runNative(file, dir);
  if (typeof native === "string") throw new Error(native);
  const js = compileJs(file, dir, "2024", false);
  if ("error" in js) throw new Error(js.error);
  const ran = runJs(runtimes[0][1], js.js, dir, "case");
  expect([native.stdout, native.outcome]).toEqual(["x half", { panic: "in the middle" }]);
  expect([ran.stdout, ran.outcome]).toEqual(["", { panic: "in the middle" }]);
}, underLoad(120_000));
