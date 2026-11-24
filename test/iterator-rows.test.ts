// What a change to the iterators' code names of the iterator matrix's
// rows, which it runs before it lands (scripts/iteratorRows.ts, #818):
// each hunk's adapters by its identifiers and what its comments quote, of
// the names the generator gives them.

import { expect, test } from "bun:test";

import { budgetOf, withoutForeignCalls, enclosingArm, entriesOf, entryAdapters, named, naming, outsideFunctions, pattern, pickedOf, pickedPerBatch, reached, rowsPerPart, type Hunk } from "../scripts/iteratorRows";
import { tableOf } from "./matrix-tables";
import { adapterCalls } from "./iterator-matrix";

const adapters = new Set(["map", "filter", "skip", "take", "fuse", "enumerate", "chain", "rev", "step_by", "zip", "take_while", "copied", "cloned"]);
const consumers = new Set(["fold", "rfold", "any", "all", "count", "nth", "for", "nth_back"]);
const hunk = (added: string[], removed: string[] = [], head = "", file = "crates/rust-js-program/src/runtime/back_steps.js"): Hunk => ({ file, at: 1, added, removed, head });

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
  expect(named(hunk(["  dropped.each = drop?.each;"], [], "function $givenDrop(drop) {", "crates/rust-js-program/src/runtime/iterator.js"), adapters, consumers)).toMatchObject({ generic: true, shared: true });
  expect(named(hunk(["x"], [], "", "crates/rust-js-program/src/runtime/forward.js"), adapters, consumers).shared).toBe(true);
  expect(named(hunk(["        return enumerated;"], ["        let item = it.item();", "        return enumerated(item);"]), adapters, consumers)).toMatchObject({ adapters: ["enumerate"], removes: true });
});

// #2199's push read code moved as a removal: three arms' `.filter(present)`
// gone into one closure beside them, rustfmt folding each call onto one
// line, ran each of 84,036 rows of `filter`, `map` and `flatten`, 1,768 s.
// What the change says again in that file was moved; #823's removed rule
// (766055774), whose `iterator_item` nothing put back, is still a removal.
test("code a change says again in its file was moved, not removed: only what's put back nowhere runs each row", () => {
  const file = "crates/rust-js-lower/src/combinators/adapters.rs";
  const folded = [
    "            IterComb::FilterMap => unboxed(",
    '                method(method(items, "map", vec![next()]), "filter", vec![present()]),',
    "                kept,",
    "            ),",
  ];
  const arm = hunk(['            IterComb::FilterMap => unboxed(somes(method(items, "map", vec![next()])), kept),'], folded, "", file);
  const closure = hunk(['            false => method(made, "filter", vec![present()]),'], [], "", file);
  expect(named(arm, adapters, consumers).removes).toBe(true);
  expect(named(arm, adapters, consumers, new Set(closure.added.flatMap((line) => line.match(/\w+/g) ?? []))).removes).toBe(false);
  const why = (found: Hunk[]) => pickedOf(found, 8).flatMap((p) => p.why);
  expect(why([closure, arm]).some((line) => line.includes("removes"))).toBe(false);
  // Said again in another file is no help: removed from this one.
  expect(why([{ ...closure, file: "crates/rust-js-lower/src/iterators.rs" }, arm]).some((line) => line.includes("removes"))).toBe(true);
  const rule = hunk(
    ["        return (dropping_items(recognition, source.peel_refs(), &drops) || enumerates_a_compiled_struct(tcx, adapter))"],
    ["        let item = recognition.iterator_item(adapter);", "        return (item.is_some_and(&drops)", "            || dropping_items(recognition, source.peel_refs(), &drops)"],
    "fn back_stepped_adapter<'tcx>(",
    "crates/rust-js-lower/src/analysis/back_stepped.rs",
  );
  expect(why([rule]).some((line) => line.includes("enumerate (removes: each row)"))).toBe(true);
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
  const decided = named(hunk(['pub(crate) const HELD_FLIPPED: [&str; 2] = ["enumerate", "map"];'], [], "", "crates/rust-js-lower/src/body_queries/in_place.rs"), adapters, consumers);
  expect(decided).toMatchObject({ adapters: ["enumerate", "map"], decides: true, removes: false });
  expect(named(hunk(["  const taking = $taking(iter, n, drop);"]), adapters, consumers).decides).toBe(false);
  // Its `HELD_FLIPPED`, outside any function, names what it says; a
  // comment there, or a `const` elsewhere, nothing.
  const outside = (added: string[], file: string): Hunk => ({ ...hunk(added, [], "fn decide_held() {", file), outside: true });
  expect(naming(outside(['const HELD_FLIPPED: [&str; 2] = ["enumerate", "map"];'], "crates/rust-js-lower/src/body_queries/in_place.rs"), adapters, consumers)).toMatchObject({ head: "" });
  expect(naming(outside(["// what a `map` decides"], "crates/rust-js-lower/src/body_queries/in_place.rs"), adapters, consumers)).toBeUndefined();
  expect(naming(outside(['const NAMES: [&str; 1] = ["map"];'], "crates/rust-js-program/src/runtime/back_steps.js"), adapters, consumers)).toBeUndefined();
  expect(adapterCalls().get("flatten")).toEqual(new Set(["map"]));
  const reach = reached(new Set(["map"]));
  expect([...reach].sort()).toEqual(["flatten", "map"]);
  // #1780's wrong answers: nothing folded, and a drop run twice more, each
  // row it failed (lily agent's `pairs` run) and the one it was found by.
  const failed = ["take.flatten | rfold | stage", "chain.flatten | rfold | next", "flatten.flatten | rfold | next", "flatten.flatten | rfold | stage", "step_by.flatten | rfold | none", "chain.flatten | rfold | none"];
  for (const id of failed.map((row) => `generic own Every | ${row}`)) {
    expect(pattern(reach, new Set(), false)!.test(id)).toBe(true);
  }
  expect(pattern(new Set(["map"]), new Set(), false)!.test("generic own Every | chain.flatten | rfold | none")).toBe(false);
});

// A family's flip adds one entry to `HELD_FLIPPED`'s line, which named
// every adapter on it: each row of all of them, 153,741 rows and 32 min 44
// s of the zip flip's check, alone (dev-loop, ioCF). What a list's entries
// alone change decides the rows of those its diff adds or removes, and of
// what reaches them; a change to anything else of it, each it names.
test("a list's entries alone decide the rows of those its diff changes, and what reaches them", () => {
  const flipped = (line: string) => `pub(crate) const HELD_FLIPPED: ${line};`;
  const file = "crates/rust-js-lower/src/body_queries/in_place.rs";
  const entry = { ...hunk([flipped('[&str; 4] = ["chain", "enumerate", "map", "zip"]')], [flipped('[&str; 3] = ["chain", "enumerate", "map"]')], "", file), outside: true };
  expect(entriesOf(entry)).toMatchObject({ added: ['"zip"'], removed: [] });
  expect(named(entriesOf(entry)!, adapters, consumers)).toMatchObject({ adapters: ["zip"], decides: true });
  // One taken out is its rows too, as a removal's.
  expect(entriesOf({ ...entry, added: entry.removed, removed: entry.added })).toMatchObject({ added: [], removed: ['"zip"'] });
  // Lines each an entry: what each changed line says, its `bool` too.
  const rows = hunk(['    ("Zip", "zip", true),'], ['    ("Zip", "zip", false),'], "", file);
  expect(entriesOf(rows)).toMatchObject({ added: ['"Zip"', '"zip"'], removed: ['"Zip"', '"zip"'] });
  expect(entriesOf(hunk(['    ("Zip", "zip", false),', '    ("Chain", "chain", false),'], ['    ("Chain", "chain", false),'], "", file))).toMatchObject({ added: ['"Zip"', '"zip"'], removed: [] });
  // Its doc beside it, rewritten in the same hunk, names nothing: the rev
  // flip's (e91e32866) ran each row of the seven families on its line.
  const doc = ["/// `flatten()`, `peekable()` and `rev()`, the families after them."];
  const documented = { ...entry, added: [...doc, ...entry.added], removed: ["/// `flatten()` and `peekable()`.", ...entry.removed] };
  expect(entriesOf(documented)).toMatchObject({ added: ['"zip"'], removed: [] });
  // Laid out again by rustfmt, one line to one an entry: its entries still,
  // as the flat_map flip's eighth was.
  const laidOut = [`pub(crate) const HELD_FLIPPED: [&str; 5] = [`, ...["chain", "enumerate", "map", "rev", "zip"].map((name) => `    "${name}",`), "];"];
  const fifth = { ...entry, added: laidOut, removed: [flipped('[&str; 4] = ["chain", "enumerate", "map", "zip"]')] };
  expect(entriesOf(fifth)).toMatchObject({ added: ['"rev"'], removed: [] });
  // Its type, its name or its visibility changed, or code beside it: the
  // whole hunk's.
  expect(entriesOf(hunk([flipped('[&str; 4] = ["chain", "enumerate", "map", "zip"]').replace("pub(crate) ", "")], entry.removed, "", file))).toBeUndefined();
  expect(entriesOf(hunk(['        ExprKind::Call { .. } => ground.contains(&given),'], ['        ExprKind::Call { .. } => false,'], "", file))).toBeUndefined();
  // Of the tables: the entry's rows and what calls it, `flatten`'s of `map`,
  // and not another's on the line; the whole line's of a change to more.
  const mapped = { ...hunk([flipped('[&str; 2] = ["chain", "map"]')], [flipped('[&str; 1] = ["chain"]')], "", file), outside: true };
  const setting = (found: Hunk) => new RegExp(pickedOf([found], 8).find((p) => p.table === "matrix")!.setting.slice(1, -1));
  const entries = setting(mapped);
  expect(["vec | map | fold | none", "vec | chain.flatten | rfold | none"].every((id) => entries.test(id))).toBe(true);
  expect(entries.test("vec | chain | fold | none")).toBe(false);
  const whole = setting({ ...mapped, added: [mapped.added[0].replace("pub(crate) ", "")] });
  expect(whole.test("vec | chain | fold | none")).toBe(true);
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
    file: "crates/rust-js-mir/src/iter.rs",
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
    file: "crates/rust-js-program/src/runtime/capacity.js",
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

// The filter_map flip's one entry, `"filter_map",` of in_place.rs's list
// (009fdbb06): its words, `filter` and `map`, and what calls them,
// `flatten`, ran 84,036 of the matrix's rows and 12,172 of the
// conformance table's. A string entry names its adapter whole.
test("a list's string entry names its adapter whole, never its words", () => {
  const flip: Hunk = { file: "crates/rust-js-lower/src/body_queries/in_place.rs", at: 113, added: ['    "filter_map",'], removed: [], head: "", outside: true };
  const entries = entriesOf(flip)!;
  expect(entries.added).toEqual(['"filter_map"']);
  const names = new Set([...adapters, "filter_map", "flatten", "flat_map"]);
  expect(entryAdapters(entries, names)).toEqual(["filter_map"]);
  // And through what calls it, nothing more: its words reached `flatten`.
  expect([...reached(new Set(entryAdapters(entries, names)))]).toEqual(["filter_map"]);
  // One no adapter is named whole by, its words as before.
  expect(entryAdapters({ ...entries, added: ['"filter maps"'] }, names)).toBeUndefined();
});

// A compile's and rustc's start are paid once a program: a pick's programs
// are of `pickedPerBatch` cases, the tables' own slices of 60, the batches
// their known cases and kept runs are of (#2340).
test("a change's rows run in programs of the rows' owner's size, the tables' slices in theirs", () => {
  for (const table of ["matrix", "conformance"] as const) {
    const of = tableOf(table);
    const cases = of.cases().slice(0, pickedPerBatch * 2);
    expect(of.batches(cases, true)[0].length).toBe(pickedPerBatch);
    expect(of.batches(cases)[0].length).toBe(60);
  }
  expect(pickedPerBatch).toBeGreaterThan(60);
});

// A method the compiler's Rust calls that it doesn't define, std's `.map(`
// or `.filter_map(`, rustc's `.skip_binder()`, is the compiler's own work and
// names no row (#2340, Tu's decision): #2331's hunk of `assoc_iterator_as_it_is`
// ran each row of `filter`, `filter_map`, `flatten`, `map` and `skip` for
// `.skip_binder().iter().filter_map(..).any(..)`. One it defines still names
// them, and so does any other spelling, a path, a helper or a string.
test("a method the compiler's Rust calls and doesn't define names no row; one it defines does", () => {
  const rust = "crates/rust-js-lower/src/iterators.rs";
  const defined = new Set(["lower_map", "stepped_by"]);
  const of = (line: string) => named(hunk([line], [], "", rust), adapters, consumers, new Set(), defined);
  expect(of("        && (tcx.item_bounds(item).skip_binder().iter()).filter_map(|b| b.as_trait_clause()).any(|p| ok(p))")).toMatchObject({ adapters: [], consumers: [] });
  expect(of("    items.iter().map(|x| x.len()).count()")).toMatchObject({ adapters: [], consumers: [] });
  expect(of("    let ty = sig.skip_binder::<Ty>(x);").adapters).toEqual([]);
  expect(of("    self.lower_map(it)").adapters).toEqual(["map"]);
  expect(of("    Some(IterMethod::StepBy) => self.stepped_by(it),").adapters).toEqual(["step_by"]);
  expect(of('    let name = "map";').adapters).toEqual(["map"]);
  // The runtime's JS, not the compiler's Rust, as it is.
  expect(named(hunk(["  return it.map((x) => x);"]), adapters, consumers, new Set(), defined).adapters).toEqual(["map"]);
  expect(withoutForeignCalls("a.map(f).lower_map(g)", defined)).toBe("a.(f).lower_map(g)");
  // A method of the compiler's own named as a consumer is, `any(&self)`: none.
  expect(named(hunk(["    if facts.any() {"], [], "", rust), adapters, consumers, new Set(), new Set(["any"])).consumers).toEqual([]);
});

// CI's nets-iterators runs the rows each push to `main` names, since the
// commit it moved `main` from, `RUST_JS_ROWS_BASE` (#2340, Tu's decision):
// the picks of the push's range, not of a branch's merge base, which on
// `main` itself is the commit checked out and names nothing.
test("the rows the change names are of the range a given base starts, a push's", async () => {
  const { changeBase } = await import("../scripts/iteratorSlice");
  expect(changeBase({ RUST_JS_ROWS_BASE: "ded89d248" })).toBe("ded89d248");
});
