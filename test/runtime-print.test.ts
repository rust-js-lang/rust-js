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

// std's `Debug` builders, step by step through a writer's sink (#1533,
// #1557): each piece written as it's taken, outside a line the sink a
// string of the writer's own. A step's `Err` keeps what its part wrote,
// through `{:#?}`'s adapter, and no later step is taken, its value never
// made, though `entries` steps on; `finish` throws it. Core's
// `fmt/builders.rs` is the model; the corpus's grids hold it to native
// Rust, this its own edges.
test("a streamed builder writes as std's, and stops its steps at an error", () => {
  const module = join(fixture("runtime-debug-builders"), "runtime.js");
  writeFileSync(module, run([compiler, "--runtime-module"]));
  const script = `
const { $debugList, $debugMap, $debugStruct, $debugTuple, $sink, $fmtError } = await import(${JSON.stringify(module)});
const seen = [];
const shown = (make) => {
  const f = $sink();
  try {
    make(f);
    seen.push("ok " + JSON.stringify(f.done()));
  } catch (error) {
    seen.push("err " + JSON.stringify(f.done() + error.written));
  }
};
const bad = () => {
  try { $fmtError(); } catch (error) { error.written = "B\\nb"; throw error; }
};
shown((f) => { const s = $debugStruct(f, "S", true); s.field("a", () => "1"); s.field("b", bad); s.field("c", () => seen.push("shown c")); s.finish(); });
shown((f) => { const l = $debugList(f); l.entry(bad); l.entries([1, 2], (n) => { seen.push("shown " + n); return String(n); }, (n) => seen.push("drop " + n)); l.finish(); });
shown((f) => { const t = $debugTuple(f, ""); t.field(() => "1"); t.finish(); });
shown((f) => { const t = $debugTuple(f, "", true); t.field(() => "1"); t.finishNonExhaustive(); });
shown((f) => { const m = $debugMap(f, true); m.entry(() => "k", () => "v\\nw"); m.finishNonExhaustive(); });
shown((f) => { const s = $debugStruct(f, "E"); s.finishNonExhaustive(); });
console.log(seen.join("|"));
`;
  expect(run([node ?? "node", "--input-type=module", "--eval", script])).toBe(
    'err "S {\\n    a: 1,\\n    b: B\\n    b"|drop 1|drop 2|err "[B\\nb"|ok "(1,)"|ok "(\\n    1,\\n    ..\\n)"|ok "{\\n    k: v\\n    w,\\n    ..\\n}"|ok "E { .. }"\n',
  );
});

// A stdout line a writer's print ends is written there and then, as std's
// `LineWriter` writes it, before a write to standard error the writer makes
// after it; and left out when the writer's text reaches the line (#1490):
// of a writer inside another, and of one that throws after it.
test("stdout's line ended inside a writer is written ahead of standard error", () => {
  const module = join(fixture("runtime-line-ahead"), "runtime.js");
  writeFileSync(module, run([compiler, "--runtime-module"]));
  const script = `
const { $out, $printing, $sink, $eprint } = await import(${JSON.stringify(module)});
const seen = [];
const shown = process.stdout.write.bind(process.stdout);
process.stdout.write = (text) => (seen.push("out " + JSON.stringify(String(text))), true);
process.stderr.write = (text) => (seen.push("err " + JSON.stringify(String(text))), true);
const noisy = (n) => {
  const f = $sink();
  f.write("<");
  $out("in " + n + "\\n");
  $eprint("err " + n + "\\n");
  f.write(n + ">");
  return f.done();
};
$printing(() => {
  $out("a ");
  $out(noisy(1));
  $out(" b\\n");
});
$printing(() => {
  $out("o[");
  const outer = $sink();
  outer.write("x");
  outer.write(noisy(2));
  $eprint("after\\n");
  outer.write("]");
  $out(outer.done());
  $out("\\n");
});
try {
  $printing(() => {
    $out("p ");
    const f = $sink();
    f.write("<");
    $out("in 3\\n");
    $eprint("err 3\\n");
    throw new Error("stopped");
  });
} catch {}
$printing(() => $out(" then\\n"));
shown(seen.join("|") + "\\n");
`;
  expect(run([node ?? "node", "--input-type=module", "--eval", script])).toBe(
    [
      'out "a <in 1\\n"', 'err "err 1\\n"', 'out "1> b\\n"',
      'out "o[x<in 2\\n"', 'err "err 2\\n"', 'err "after\\n"', 'out "2>]\\n"',
      'out "p <in 3\\n"', 'err "err 3\\n"', 'out " then\\n"',
    ].join("|") + "\n",
  );
});

