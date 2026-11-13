// The generic facts' rules of the architecture, split from
// architecture.test.ts at its cap (a test file is at most 1500 lines): what
// generic code needs of its type parameters is one catalogue (ADR 1115).
import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";

const read = (path: string) => readFileSync(join(root, path), "utf8");
// In name order, as macOS lists them and Linux, CI's, doesn't.
const files = (directory: string): string[] => readdirSync(join(root, directory), { withFileTypes: true })
  .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
  .flatMap(entry => entry.isDirectory() ? files(`${directory}/${entry.name}`) : [`${directory}/${entry.name}`]);

// What generic code needs of its type parameters is one catalogue,
// `generic_facts.rs`' table, each fact's route and owner (ADR 1115). Each
// file's reads of a type parameter, `ty::Param(`, `has_param()` and
// `is_unknown(`, are the audit's of 2026-11-10 at most: a ratchet, which a
// migration onto the catalogue lowers. A new read asks the fact's owner the
// catalogue names, or adds its fact to the catalogue first.
const typeParameterReads: Record<string, number> = Object.fromEntries(
  read("test/type-parameter-reads.txt").split("\n").filter((line) => line && !line.startsWith("#"))
    .map((line) => line.split("\t")).map(([file, n]) => [file, Number(n)]),
);

test("a type parameter's facts are asked of their owner", () => {
  const counted = (text: string) =>
    text.split("\n").filter((line) => !line.trim().startsWith("//"))
      .reduce((n, line) => n + (line.match(/ty::Param\(|has_param\(\)|is_unknown\(/g) ?? []).length, 0);
  // The catalogue's own file asks them for its owners.
  const grown = files("src/lower").filter((file) => file.endsWith(".rs") && file !== "src/lower/generic_facts.rs")
    .map((file) => [file, counted(read(file)), typeParameterReads[file] ?? 0] as const)
    .filter(([, now, was]) => now > was)
    .map(([file, now, was]) => `${file}: ${now} reads of a type parameter, the audit's ${was}: ask the owner src/lower/generic_facts.rs names`);
  expect(grown).toEqual([]);
});

test("each fact in the generic facts' catalogue names an owner that's there", () => {
  const rows = read("src/lower/generic_facts.rs").split("\n")
    .map((line) => line.match(/^\/\/! \|.*\| (Given|Form|Instances) \| `(\S+) (\w+)` \|$/))
    .filter((match) => match !== null);
  expect(rows.length).toBeGreaterThan(10);
  const missing = rows.filter(([, , file, fn]) => !new RegExp(`fn ${fn}\\b`).test(read(file)))
    .map(([, , file, fn]) => `${file} ${fn}`);
  expect(missing).toEqual([]);
});

// A size std grows a `Vec` by and checks its room by, `TypeFact::Room`, is
// given as `undefined`, its growth not kept and its room counted alone, of
// #934's case alone: an associated type no caller gives and no dictionary
// carries, an impl's `I::Item` (#1260, the user's decision). `room_value` is
// the one owner of `Room`'s value and the one place it's `undefined`, gated
// by `is_associated_type`; an impl's method takes no `Room` of an item
// slot, which its dictionary wouldn't pass; and no other place gives `Room`
// as `undefined`, so the fallback can't spread.
test("a Room size is undefined only of #934's case, by room_value", () => {
  const body = (text: string, name: string) => {
    const at = text.indexOf(`fn ${name}(`);
    return at < 0 ? "" : text.slice(at, text.indexOf("\n    }\n", at));
  };
  const code = (file: string) => read(file).split("\n").filter((line) => !line.trim().startsWith("//"));
  const rust = files("src/lower").filter((file) => file.endsWith(".rs"));
  const room = body(read("src/lower/iterators/hint_asks.rs"), "room_value");
  expect(room).toContain("(crate::lower::recognition::is_associated_type(item) && self.in_impl_method()).then(Expr::undefined)");
  expect(room.match(/undefined/g)?.length).toBe(1);
  const gated = rust.flatMap((file) => code(file).filter((line) => line.includes("is_associated_type(") && !line.includes("fn is_associated_type(")).map(() => file));
  expect(gated).toEqual(["src/lower/iterators/hint_asks.rs"]);
  const elsewhere = rust.filter((file) => code(file).some((line) => /TypeFact::Room/.test(line) && /undefined/.test(line)));
  expect(elsewhere).toEqual([]);
  expect(read("src/lower/analysis/type_facts.rs")).toMatch(/fact == TypeFact::Room\s*&& tcx\.trait_impl_of_assoc\(id\)\.is_some\(\)\s*&& index >= tcx\.generics_of\(id\)\.count\(\) as u32/);
});

// A generic function takes only the size facts it reads: a collect's
// \`Room\` is asked only of one counted by the program, whose room check
// reads it (\`counted_by_program\`, #1436), so no function gains a \`TSize\`
// it never reads. Each corpus snapshot's functions are scanned; the one
// listed is #1443's, which predates the rule, and the list only shrinks.
test("a generic function takes no size fact it doesn't read", () => {
  const known = [
    "impl_asks_the_layout_of_a_type_of_its_parameters.js hashMapArbitrary_shrink KVSize",
    // A tuple key relayed a level deeper each round, to the bound (#1708).
    "map_keyed_by_a_tuple_of_a_type_parameter_derived_default_copy.js iTools_positionsSeen UsizeUsizeUsizeUsizeUsizeUsizeUsizeIItemSize",
    "map_keyed_by_a_tuple_of_a_type_parameter_derived_default_copy.js iTools_positionsSeen UsizeUsizeUsizeUsizeUsizeUsizeIItemSize",
    "map_keyed_by_a_tuple_of_a_type_parameter_derived_default_copy.js iTools_positionsSeen UsizeUsizeUsizeUsizeUsizeIItemSize",
    "map_keyed_by_a_tuple_of_a_type_parameter_derived_default_copy.js iTools_positionsSeen UsizeUsizeUsizeUsizeIItemSize",
    "map_keyed_by_a_tuple_of_a_type_parameter_derived_default_copy.js iTools_positionsSeen UsizeUsizeUsizeIItemSize",
    "map_keyed_by_a_tuple_of_a_type_parameter_derived_default_copy.js iTools_positionsSeen UsizeUsizeIItemSize",
  ];
  const unread = files("test/corpus").filter((file) => file.endsWith(".js")).flatMap((file) => {
    const text = read(file);
    return [...text.matchAll(/^(?:export )?function (\w+)\(([^)]*)\) \{\n([\s\S]*?)^\}/gm)].flatMap(([, name, params, body]) =>
      params.split(",").map((param) => param.trim())
        .filter((param) => /^\w*Size$/.test(param) && !new RegExp(`\\b${param}\\b`).test(body))
        .map((param) => `${file.slice("test/corpus/".length)} ${name} ${param}`));
  });
  expect(unread.filter((each) => !known.includes(each))).toEqual([]);
  expect(known.filter((each) => !unread.includes(each)), "fixed: take it off the list (#1443)").toEqual([]);
});
