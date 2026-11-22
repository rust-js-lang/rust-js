// Where a compile's time goes (#2296): a compile given `RUST_JS_PHASE_TIMES`
// appends a line of its phases' times to the file it names, rustc's front
// end, lowering and the back end in turn, and one not given it writes
// nothing; `scripts/phase-times.ts` sums such lines into tables.

import { expect, test } from "bun:test";
import { readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
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
  expect(line.forced).toEqual([]);
  // Not given it, nothing more is appended to the file.
  const plain = runSync([compiler, join(dir, "main.rs"), "-o", join(dir, "plain.js")], root, 60_000, { RUST_JS_PHASE_TIMES: undefined });
  expect(plain.code, plain.stderr).toBe(0);
  expect(readFileSync(times, "utf8").split("\n").filter(Boolean)).toHaveLength(1);
});

// rustc's walk of every crate's modules, `visible_parent_map`, which
// printing a type or a path forces, was 37 ms of each of the React port's
// compiles, forced by std's recognizers and by a type printed for a testing
// knob (#2314). A lowering asks it of none: a compile of std's iterators,
// adapters and strings, a struct written in place, says its lowering forced
// none, though rustc's warnings walked it first; one that prints a type,
// the knob's of std's adapters, says it did, so a print on a hot path is
// seen. And no source asks rustc's map: std's own is `std_paths`'.
test("a lowering forces no walk of every crate's modules, and a timed compile says where one does", () => {
  const dir = fixture("phase-times-forced");
  const input = join(root, "test/corpus/adapter_of_a_value_given_as_it_is.rs");
  writeFileSync(join(dir, "main.rs"), [
    "struct Pair { a: i32 }",
    "fn main() {",
    "    let mut p = Pair { a: 1 };",
    "    p.a += 1;",
    "    let v: Vec<i32> = (0..5).map(|x| x * p.a).filter(|x| x % 2 == 0).collect();",
    '    let s: String = "abc".chars().rev().collect();',
    '    println!("{v:?} {s} {}", v.iter().copied().sum::<i32>());',
    "}",
    "",
  ].join("\n"));
  const times = join(dir, "times.jsonl");
  const forced = (args: string[], rustc: string[] = []) => {
    writeFileSync(times, "");
    runSync([compiler, ...args, "-o", join(dir, "out.js"), "--", "--edition=2024", ...rustc], root, 60_000, { RUST_JS_PHASE_TIMES: times });
    return readFileSync(times, "utf8").split("\n").filter(Boolean).map((text) => JSON.parse(text).forced);
  };
  const quiet = ["-A", "warnings"];
  expect(forced([join(dir, "main.rs")], quiet)).toEqual([[]]);
  expect(forced([input], quiet)).toEqual([[]]);
  // Its drops compared with rustc's elaboration print their function's path
  // only where one is said.
  expect(forced([join(root, "test/corpus/adapter_drops.rs")], quiet)).toEqual([[]]);
  // The knob given, each type std's adapters write in place is printed.
  const knob = ["--without-mutated", "Forgotten", input];
  expect(forced(knob, quiet)).toEqual([["visible_parent_map"]]);
  // Not where rustc's warnings walked it first: a result unused, its type
  // printed, `std::vec::Vec<u8>`, as the crate's analysis ends.
  const warned = join(dir, "warned.rs");
  writeFileSync(warned, `${readFileSync(input, "utf8")}\npub fn unused() {\n    let mut v = vec![1u8];\n    std::mem::take(&mut v);\n}\n`);
  expect(forced(["--without-mutated", "Forgotten", warned], ["-W", "unused_results"])).toEqual([[]]);
  // Nor where a refusal's words were made before it was known to refuse,
  // which printed a type or a path in compiles that refuse nothing: a
  // trait's dictionary through a box, one generic code holds in a place of
  // its own, a counted `Rc` made a `dyn`, two `Rc`s compared, and an item's
  // built types ordered by their text where no two of one item tie (#2321).
  for (const name of ["as_ref_and_borrow_through_a_mut", "copy_mutation", "dyn_any_counted_rc", "rc_eq_by_address_first", "clone_from_generic_std_own"]) {
    expect([name, forced([join(root, "test/corpus", `${name}.rs`)], quiet)]).toEqual([name, [[]]]);
  }
  const sources = (at: string): string[] =>
    readdirSync(at).flatMap((name) => (statSync(join(at, name)).isDirectory() ? sources(join(at, name)) : name.endsWith(".rs") ? [join(at, name)] : []));
  const asking = [...sources(join(root, "src")), ...sources(join(root, "crates"))].filter((file) => readFileSync(file, "utf8").includes("visible_parent_map(")).map((file) => file.slice(root.length + 1));
  expect(asking).toEqual([]);
});

test("the report sums a case's steps and a compile's phases", () => {
  const text = [
    JSON.stringify({ case: "a.rs", total: 300, steps: [["compile", 200], ["native run", 100]] }),
    JSON.stringify({ input: "corpus-a_0/lib.rs", library: false, phases: [["lowering", 120], ["write", 30]], forced: ["visible_parent_map"] }),
    JSON.stringify({ input: "corpus-a_0/lib.rs", library: true, phases: [["lowering", 10]], forced: [] }),
  ].join("\n");
  const tables = report(text);
  expect(tables).toContain("1 cases, 0.3 s summed");
  expect(tables).toContain("| compile | 0.2 | 66.7% | 1 | 200 | 200 |");
  expect(tables).toContain("| lowering | 0.1 | 60.0% | 1 | 120 | 120 |");
  expect(tables).toContain("| process start and exit | 0.1 | 25.0% | 1 | 50 | 50 |");
  expect(tables).toContain("rustc's walks of every crate lowering forced: visible_parent_map by 1 of 2 compiles");
});
