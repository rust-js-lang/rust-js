// Where a compile's time goes (#2296): a compile given `RUST_JS_PHASE_TIMES`
// appends a line of its phases' times to the file it names, rustc's front
// end, lowering and the back end in turn, and one not given it writes
// nothing; `scripts/phase-times.ts` sums such lines into tables.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { report } from "../scripts/phase-times";
import { compiler, fixture, root } from "./support";
import { runSync } from "./child";

test("a timed compile appends its phases, and an untimed one writes nothing", () => {
  const dir = fixture("phase-times");
  writeFileSync(join(dir, "main.rs"), 'fn main() {\n    println!("{}", 1 + 2);\n}\n');
  const times = join(dir, "times.jsonl");
  const timed = runSync([compiler, join(dir, "main.rs"), "-o", join(dir, "main.js")], root, 60_000, { RUST_JS_PHASE_TIMES: times });
  expect(timed.code, timed.stderr).toBe(0);
  const [line, ...more] = readFileSync(times, "utf8").split("\n").filter(Boolean).map((text) => JSON.parse(text));
  expect(more).toEqual([]);
  expect(line.input).toBe(join(dir, "main.rs"));
  expect(line.library).toBe(false);
  expect(line.phases.map(([phase]: [string, number]) => phase)).toEqual([
    "rustc: parse and expand",
    "rustc: type check, MIR and borrow check",
    "lowering",
    "link",
    "rustc: the rest",
    "prepare and print",
    "write",
    "the crate's checks",
  ]);
  expect(line.phases.every(([, ms]: [string, number]) => ms >= 0)).toBe(true);
  // Not given it, nothing more is appended to the file.
  const plain = runSync([compiler, join(dir, "main.rs"), "-o", join(dir, "plain.js")], root, 60_000, { RUST_JS_PHASE_TIMES: undefined });
  expect(plain.code, plain.stderr).toBe(0);
  expect(readFileSync(times, "utf8").split("\n").filter(Boolean)).toHaveLength(1);
});

test("the report sums a case's steps and a compile's phases", () => {
  const text = [
    JSON.stringify({ case: "a.rs", total: 300, steps: [["compile", 200], ["native run", 100]] }),
    JSON.stringify({ input: "corpus-a_0/lib.rs", library: false, phases: [["lowering", 120], ["write", 30]] }),
  ].join("\n");
  const tables = report(text);
  expect(tables).toContain("1 cases, 0.3 s summed");
  expect(tables).toContain("| compile | 0.2 | 66.7% | 1 | 200 | 200 |");
  expect(tables).toContain("| lowering | 0.1 | 60.0% | 1 | 120 | 120 |");
  expect(tables).toContain("| process start and exit | 0.1 | 25.0% | 1 | 50 | 50 |");
});
