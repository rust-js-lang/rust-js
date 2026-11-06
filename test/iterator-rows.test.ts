// What a change to the iterators' code names of the iterator matrix's
// rows, which it runs before it lands (scripts/iteratorRows.ts, #818):
// each hunk's adapters by its identifiers and what its comments quote, of
// the names the generator gives them.

import { expect, test } from "bun:test";

import { enclosingArm, named, pattern, type Hunk } from "../scripts/iteratorRows";

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

