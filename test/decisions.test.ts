// Decision coverage's catalogue (src/decisions.rs) is the one list of the
// decisions counted and their answers: each is recorded by the owner it
// names, and each record names a decision and answers of it, so no count
// is of a name the report doesn't know, and no catalogued answer is one no
// code gives (docs/finding-wrong-answers.md).
import { expect, test } from "bun:test";
import { readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { underLoad } from "../scripts/timing";
import { compiler, fixture, root, run } from "./support";

const catalogue = () => {
  const text = readFileSync(join(root, "src/decisions.rs"), "utf8");
  const entries = [...text.matchAll(/name: "([^"]+)",\s*owner: "([^" ]+) [^"]*",\s*answers: &\[([^\]]*)\],\s*typed: (true|false)/g)];
  return entries.map(([, name, file, answers, typed]) => ({
    name,
    file,
    answers: [...answers.matchAll(/"([^"]+)"/g)].map(([, a]) => a),
    typed: typed === "true",
  }));
};

const sources = (dir: string): string[] =>
  readdirSync(join(root, dir)).flatMap((entry) => {
    const path = join(dir, entry);
    return statSync(join(root, path)).isDirectory() ? sources(path) : path.endsWith(".rs") ? [path] : [];
  });

test("each catalogued decision is recorded by its owner", () => {
  const decisions = catalogue();
  expect(decisions.length).toBeGreaterThan(10);
  const unrecorded = decisions
    .filter(({ name, file, typed }) => !new RegExp(`decisions::record${typed ? "_typed" : ""}\\(\\s*"${name}"`).test(readFileSync(join(root, file), "utf8")))
    .map(({ name, file }) => `${name}: ${file} records none`);
  expect(unrecorded).toEqual([]);
});

test("each record names a catalogued decision and its answers", () => {
  const decisions = new Map(catalogue().map((d) => [d.name, d.answers]));
  const wrong: string[] = [];
  for (const file of sources("src")) {
    const text = readFileSync(join(root, file), "utf8");
    // A record's decision, and each answer it gives in its call or the
    // answer `match` that feeds it, as literals: of an answer a local
    // holds, each value its `let` gives, a literal not an argument of a
    // call (`unreachable!("..")`'s).
    for (const record of text.matchAll(/decisions::record(?:_typed)?\(\s*"([^"]+)",\s*([^;]*)\);/g)) {
      const [call, name, rest] = record;
      const answers = decisions.get(name);
      if (!answers) {
        wrong.push(`${file}: ${name} isn't catalogued`);
        continue;
      }
      const local = rest.match(/^([a-z_][a-z0-9_]*)\s*[,)]/)?.[1];
      const given = local ? text.slice(text.lastIndexOf(`let ${local} =`, record.index), record.index) : "";
      if (local && !given) wrong.push(`${file}: ${name}'s answer ${local} is given by no \`let\` before it`);
      const literals = [...call.slice(call.indexOf(",")).matchAll(/"([^"]+)"/g), ...given.matchAll(/(?<![(\[!]\s*)"([^"]+)"/g)];
      for (const [, literal] of literals) {
        if (!answers.includes(literal)) wrong.push(`${file}: ${name}'s answer ${literal} isn't catalogued`);
      }
    }
  }
  expect(wrong).toEqual([]);
});

// A compile asked to count, `RUST_JS_DECISIONS`, into a file that has
// none, writes each decision's every answer, of each kind of a typed one
// and each body, one it gave by how many times, one it didn't by `0`,
// tagged by its build and shape; one after, into the same file, only what
// it gave; one not asked writes nothing. What a body of the crate's gives
// is its own, `own`; a typed decision's by its type's kind.
test("a compile asked counts every answer of every decision, a 0 too, then what it gave", () => {
  const dir = fixture("decisions-counted");
  const source = join(dir, "main.rs");
  writeFileSync(source, "use std::cell::RefCell;\npub fn main() {\n    let c = RefCell::new(1);\n    *c.borrow_mut() += 1;\n    println!(\"{}\", c.borrow());\n}\n");
  const counts = join(dir, "counts.tsv");
  rmSync(counts, { force: true });
  const compile = () => run([compiler, source, "-o", join(dir, "main.js"), "--", "--crate-type=lib"], 120_000, { RUST_JS_DECISIONS: counts });
  compile();
  const rows = readFileSync(counts, "utf8").trim().split("\n").map((line) => line.split("\t"));
  const answers = catalogue().reduce((n, d) => n + d.answers.length * (d.typed ? 8 : 1) * 3, 0);
  expect(rows.length).toBe(answers);
  // decision, answer, kind, body, count, build, shape, owner
  const borrowed = rows.filter(([decision, , , body]) => decision === "refcell-borrow" && body === "own").reduce((n, row) => n + Number(row[4]), 0);
  expect(borrowed).toBeGreaterThan(0);
  expect(rows.find(([decision, answer, , body]) => decision === "generic-form" && answer === "held-box" && body === "own")?.slice(2, 7)).toEqual(["-", "own", "0", "plain", "program"]);
  expect(rows.some(([decision, , kind, , count]) => decision === "copy-if-needed" && kind !== "-" && Number(count) > 0)).toBe(true);
  compile();
  const after = readFileSync(counts, "utf8").trim().split("\n").slice(rows.length).map((line) => line.split("\t"));
  expect(after.length).toBeGreaterThan(0);
  expect(after.filter((row) => Number(row[4]) === 0)).toEqual([]);
}, underLoad(240_000));

// Which chains are transparent (#1673), each kept from it by the first
// of: an `Iterator` of the crate's, an iterator held apart it starts on,
// a source not an array, a string or a range, an iterator that escapes
// (`{:?}`'s of `v.iter()` too), a destructor in its items or a closure's
// captures, an iterator method no table proves (`product`), an adapter's
// argument run as it's given (`chain(more(&v))`), a `Vec` collected in a
// crate that reads capacities; `(0..10).filter(..)`, `(1..=3).map(..)`,
// whose `RangeInclusive::new` makes its source, and a `for` loop's
// `v.iter().rev()` admitted.
test("each chain is counted by what keeps it from being transparent", () => {
  const dir = fixture("decisions-transparent");
  const source = join(dir, "main.rs");
  writeFileSync(source, `use std::collections::BTreeSet;

struct Counter(u32);

impl Iterator for Counter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0 += 1;
        (self.0 < 4).then_some(self.0)
    }
}

fn evens(v: &[i32]) -> impl Iterator<Item = &i32> {
    v.iter().filter(|x| **x % 2 == 0)
}

fn more(v: &[i32]) -> std::slice::Iter<'_, i32> {
    v.iter()
}

pub fn main() {
    let v = vec![1, 2, 3, 4];
    let doubled: Vec<i32> = v.iter().map(|x| x * 2).collect();
    let thirds: i32 = (0..10).filter(|x| x % 3 == 0).sum();
    let counted: u32 = Counter(0).map(|x| x + 1).sum();
    let names = vec![String::from("a"), String::from("b")];
    let shouted: Vec<String> = names.into_iter().map(|s| s + "!").collect();
    let even = evens(&v).count();
    let set = BTreeSet::from([3, 1, 2]);
    let total: i32 = set.iter().sum();
    let tag = String::from("tag");
    let tagged: Vec<usize> = v.iter().map(move |x| *x as usize + tag.len()).collect();
    let product: i32 = v.iter().product();
    let chained = v.iter().chain(more(&v)).count();
    let kept: Vec<i32> = v.iter().copied().collect();
    println!("{product} {chained} {}", kept.capacity());
    let doubled_range: i32 = (1..=3).map(|x| x * 2).sum();
    let mut drained = vec![1, 2];
    let taken: Vec<i32> = drained.drain(..).collect();
    println!("{doubled_range} {:?} {taken:?}", v.iter());
    for x in v.iter().rev() {
        print!("{x}");
    }
    println!("{doubled:?} {thirds} {counted} {shouted:?} {even} {total} {tagged:?}");
}
`);
  const counts = join(dir, "counts.tsv");
  rmSync(counts, { force: true });
  run([compiler, source, "-o", join(dir, "main.js"), "--", "--crate-type=lib"], 120_000, { RUST_JS_DECISIONS: counts });
  const given: Record<string, number> = {};
  for (const [decision, answer, , , count] of readFileSync(counts, "utf8").trim().split("\n").map((line) => line.split("\t"))) {
    if (decision === "transparent-chain" && Number(count) > 0) given[answer] = (given[answer] ?? 0) + Number(count);
  }
  expect(given).toEqual({ "user-iterator": 1, held: 1, source: 1, escapes: 3, "drop-glue": 2, unproved: 1, order: 1, capacity: 3, admitted: 3 });
}, underLoad(120_000));

// An answer given only by a program that reaches it is counted, not
// refused as uncatalogued: `generic-form`'s `tagged`, a write's result
// given to generic code, crashed every compile the nightly counted that
// reached one (nightly 43133510260), as no check before it counted.
test("a compile asked counts generic code's tagged form of a write's result", () => {
  const dir = fixture("decisions-tagged");
  const counts = join(dir, "counts.tsv");
  rmSync(counts, { force: true });
  const source = join(root, "test/corpus/write_result_given_to_generic_code.rs");
  const compiled = Bun.spawnSync([compiler, source, "-o", join(dir, "main.js")], {
    cwd: root,
    env: { ...process.env, RUST_JS_DECISIONS: counts },
  });
  expect([compiled.exitCode, compiled.stderr.toString().match(/panicked.*\n.*/)?.[0]]).toEqual([0, undefined]);
  const tagged = readFileSync(counts, "utf8").trim().split("\n").map((line) => line.split("\t"))
    .filter(([decision, answer]) => decision === "generic-form" && answer === "tagged")
    .reduce((n, row) => n + Number(row[4]), 0);
  expect(tagged).toBeGreaterThan(0);
}, underLoad(120_000));

// A workflow gives an empty setting where it asks no counts, a run not the
// nightly's (nightly.yml, check.yml): that asks none, not a file named ``.
test("an empty setting asks no counts", () => {
  const dir = fixture("decisions-empty");
  const source = join(dir, "main.rs");
  writeFileSync(source, "pub fn main() {\n    println!(\"{}\", 1);\n}\n");
  const compiled = Bun.spawnSync([compiler, source, "-o", join(dir, "main.js"), "--", "--crate-type=lib"], {
    cwd: root,
    env: { ...process.env, RUST_JS_DECISIONS: "" },
  });
  expect([compiled.exitCode, compiled.stderr.toString()]).toEqual([0, ""]);
}, underLoad(120_000));
