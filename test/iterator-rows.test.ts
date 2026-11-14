// What a change to the iterators' code names of the iterator matrix's
// rows, which it runs before it lands (scripts/iteratorRows.ts, #818):
// each hunk's adapters by its identifiers and what its comments quote, of
// the names the generator gives them.

import { expect, test } from "bun:test";

import { budgetOf, enclosingArm, named, naming, outsideFunctions, pattern, reached, rowsPerPart, type Hunk } from "../scripts/iteratorRows";
import { adapterCalls } from "./iterator-matrix";

const adapters = new Set(["map", "filter", "skip", "take", "fuse", "enumerate", "chain", "rev", "step_by", "zip", "take_while", "copied", "cloned"]);
const consumers = new Set(["fold", "rfold", "any", "all", "count", "nth", "for", "nth_back"]);
const hunk = (added: string[], removed: string[] = [], head = "", file = "src/runtime/back_steps.js"): Hunk => ({ file, at: 1, added, removed, head });

test("a hunk names the adapters its code's identifiers are of, as they're inflected", () => {
  expect(named(hunk(["  const taking = $taking(iter, n, drop);"]), adapters, consumers).adapters).toEqual(["take"]);
  expect(named(hunk(["    Some(IterMethod::StepBy) => self.stepped_by(it),"]), adapters, consumers).adapters).toEqual(["step_by"]);
  expect(named(hunk(["", "  if (it.ownNthBack) stepped.ownNthBack = true;"], [], "function $stepByEnds(it, step, drop) {"), adapters, consumers).adapters).toEqual(["step_by"]);
});

test("a comment names nothing, nor does the language's own `for`", () => {
  expect(named(hunk(["  // any of them, all taken, each one mapped"]), adapters, consumers)).toMatchObject({ adapters: [], consumers: [] });
  expect(named(hunk(["  // what a `fuse()` gives, `rfold`ed"]), adapters, consumers)).toMatchObject({ adapters: [], consumers: [] });
  expect(named(hunk(["  for (let k = 0; k < n; k++) at = it.next();"]), adapters, consumers).consumers).not.toContain("for");
});

test("a hunk that names no adapter names its consumers", () => {
  expect(named(hunk(["  const folded = it.rfold(init, f);"]), adapters, consumers).consumers).toEqual(["rfold"]);
  expect(named(hunk(["        Some(IterMethod::Count) => self.counted(it),"]), adapters, consumers).consumers).toEqual(["count"]);
  expect(named(hunk(["  const step = it.nthBack(k);"]), adapters, consumers).consumers).toEqual(["nth", "nth_back"]);
  expect(named(hunk(["  $taking(it).rfold(init, f);"]), adapters, consumers)).toMatchObject({ adapters: ["take"], consumers: [] });
});

test("generic code's, what every adapter shares and what's taken away are said", () => {
  expect(named(hunk(["  dropped.each = drop?.each;"], [], "function $givenDrop(drop) {", "src/runtime/iterator.js"), adapters, consumers)).toMatchObject({ generic: true, shared: true });
  expect(named(hunk(["x"], [], "", "src/runtime/forward.js"), adapters, consumers).shared).toBe(true);
  expect(named(hunk(["        return enumerated;"], ["        let item = it.item();", "        return enumerated(item);"]), adapters, consumers)).toMatchObject({ adapters: ["enumerate"], removes: true });
});

test("a name's rows are those whose chain has it, or whose consumer it begins", () => {
  const take = pattern(new Set(["take"]), new Set(), false)!;
  expect(["vec | take | fold | none", "vec | map.take | rfold | drop", "vec | map holding.take | nth | none"].every((id) => take.test(id))).toBe(true);
  expect(["vec | take_while | fold | none", "vec | map | fold | take"].some((id) => take.test(id))).toBe(false);
  const rfold = pattern(new Set(), new Set(["rfold"]), false)!;
  expect(rfold.test("vec | map | rfold holding | none") && !rfold.test("vec | map | fold | none")).toBe(true);
  expect(pattern(new Set(), new Set(), true)!.test("generic vec | - | fold | none")).toBe(true);
});

// The held flip (bb518fcca) named `map` in what decides each chain's form,
// and its slice ran a sample of `map`'s rows and none of `chain.flatten`'s,
// whose per-item `map` the id doesn't name: #1780's wrong answers, which the
// matrix's `generic own Every | chain.flatten | rfold` and `take.flatten`
// rows hold.
test("what decides each chain's form runs each row of what it names, and of each adapter that calls it", () => {
  const decided = named(hunk(['pub(in crate::lower) const HELD_FLIPPED: [&str; 2] = ["enumerate", "map"];'], [], "", "src/lower/body_queries/in_place.rs"), adapters, consumers);
  expect(decided).toMatchObject({ adapters: ["enumerate", "map"], decides: true, removes: false });
  expect(named(hunk(["  const taking = $taking(iter, n, drop);"]), adapters, consumers).decides).toBe(false);
  // Its `HELD_FLIPPED`, outside any function, names what it says; a
  // comment there, or a `const` elsewhere, nothing.
  const outside = (added: string[], file: string): Hunk => ({ ...hunk(added, [], "fn decide_held() {", file), outside: true });
  expect(naming(outside(['const HELD_FLIPPED: [&str; 2] = ["enumerate", "map"];'], "src/lower/body_queries/in_place.rs"), adapters, consumers)).toMatchObject({ head: "" });
  expect(naming(outside(["// what a `map` decides"], "src/lower/body_queries/in_place.rs"), adapters, consumers)).toBeUndefined();
  expect(naming(outside(['const NAMES: [&str; 1] = ["map"];'], "src/runtime/back_steps.js"), adapters, consumers)).toBeUndefined();
  expect(adapterCalls().get("flatten")).toEqual(new Set(["map"]));
  const reach = reached(new Set(["map"]));
  expect([...reach].sort()).toEqual(["flatten", "map"]);
  // #1780's wrong answers: nothing folded, and a drop run twice more, each
  // row it failed (m6-lily's `pairs` run) and the one it was found by.
  const failed = ["take.flatten | rfold | stage", "chain.flatten | rfold | next", "flatten.flatten | rfold | next", "flatten.flatten | rfold | stage", "step_by.flatten | rfold | none", "chain.flatten | rfold | none"];
  for (const id of failed.map((row) => `generic own Every | ${row}`)) {
    expect(pattern(reach, new Set(), false)!.test(id)).toBe(true);
  }
  expect(pattern(new Set(["map"]), new Set(), false)!.test("generic own Every | chain.flatten | rfold | none")).toBe(false);
});

// #850's hunk changed only the arm's body: what the arm matches, though that
// line is the same, names its adapters.
test("a hunk names the adapters of the match arm it's in", () => {
  const lines = [
    "    fn std_method_called(&mut self, it: Expr) -> R<Option<Expr>> {",
    "        Ok(Some(match std_method {",
    "            Some(IterMethod::Copied | IterMethod::Cloned) if self.is_its_source(output) => it,",
    "            Some(IterMethod::Copied | IterMethod::Cloned) => match item {",
    "                Some(item) => self.copied_items(it, item, output, span)?,",
    "                None => return Ok(None),",
    "            },",
  ];
  const arm = enclosingArm(lines, 4);
  expect(arm).toBe(lines[3]);
  const hunk: Hunk = {
    file: "src/lower/mir/iter.rs",
    at: 5,
    added: [lines[4]],
    removed: ["                Some(item) => self.copied_items(it, item, self.knows_its_length(output), span)?,"],
    head: lines[0],
    arm,
  };
  expect(named(hunk, adapters, new Set(["fold"])).adapters).toEqual(["cloned", "copied"]);
  expect(enclosingArm(lines, 1)).toBeUndefined();
});

// #849's hunk to a runtime helper what collects share: its comment's words
// and a size's `count` name no consumer, so it runs `pairs` of all of them,
// not each row of `count` and `for`.
test("a helper's hunk names no consumer by its variables or comments", () => {
  const capacity: Hunk = {
    file: "src/runtime/capacity.js",
    at: 41,
    added: [
      "// The least capacity std's `Vec` of items of `size` bytes grows to,",
      "// `MIN_NON_ZERO_CAP` (raw_vec/mod.rs `min_non_zero_cap`).",
      "function $leastCapacity(size) {",
      "  return size === 1 ? 8 : size <= 1024 ? 4 : 1;",
      "}",
    ],
    removed: [],
    head: "function $roomFor(count, size) {",
  };
  expect(named(capacity, adapters, consumers)).toMatchObject({ adapters: [], consumers: [] });
  const counted = hunk(["  const count = $count(it, drop);", "  for (let k = 0; k < count; k++) at = $stepOf(it);"]);
  expect(named(counted, adapters, consumers).consumers).toEqual([]);
});

// The rows a change runs fit its time on whatever machine runs them: as
// many as its parts run in it, at the machine's own rate (#857: the M1's 8
// cores ran 12 cores' 16,000 and 5,000 in 4 parts in 43 to 55 min).
test("a change's rows are as many as its parts run in its time", () => {
  for (const table of ["matrix", "conformance"] as const) {
    expect(budgetOf(table, 12)).toBeGreaterThan(budgetOf(table, 4));
    expect(budgetOf(table, 4)).toBe(Math.max(1_000, Math.floor(rowsPerPart(table) * 4 * (table === "matrix" ? 150 : 40))));
  }
});

test("a hunk outside any function names no rows: a comment after the last one, or before the first", () => {
  const lines = ["// the file's head", "fn a() {", "    let s = \"}\"; // a `}` in text", "    b();", "}", "", "// after the last"];
  expect(outsideFunctions(lines, 1, 3)).toBe(false);
  expect(outsideFunctions(lines, 1, 6)).toBe(true);
  expect(outsideFunctions(lines, -1, 0)).toBe(true);
});
