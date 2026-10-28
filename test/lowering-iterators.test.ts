// What the compiler writes of iterators: chains as array methods where order
// can't be told, adapters of JS iterators, ranges, consumers that stop early,
// and the crate's own iterators. Each test is a program of its own, compiled
// and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, performanceTrack, root, run } from "./support";

beforeAll(buildReact, 600_000);

// ADR 0512: a list MIR held while it took its other items out, `[undefined,
// ids(v)]` of `println!`'s arguments, is its one item left, read as itself.
test("a list of one item left is the item", () => {
  const dir = fixture("holed-lists");
  writeFileSync(join(dir, "lib.rs"), readFileSync(join(root, "test/corpus/extract_if_drops.rs"), "utf8").replace("fn main()", "pub fn main()"));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("console.log(`odd [${value.join(\", \")}] kept [${value$1.join(\", \")}]`);");
  expect(js).not.toContain("[undefined, ids(");
});

// ADR 0508: no `.map((item) => item)`: a list of unit enums, or of numbers,
// is shown by `join` alone, and `cloned()` of strings is the strings.
test("no item is mapped to itself", () => {
  const dir = fixture("no-identity-maps");
  writeFileSync(join(dir, "enums.rs"), readFileSync(join(root, "test/corpus/enum_order.rs"), "utf8").replace("fn main()", "pub fn main()"));
  writeFileSync(join(dir, "calc.rs"), readFileSync(join(root, "test/corpus/calculator.rs"), "utf8").replace("fn main()", "pub fn main()"));
  run([compiler, join(dir, "enums.rs"), "-o", join(dir, "enums.js")]);
  run([compiler, join(dir, "calc.rs"), "-o", join(dir, "calc.js")]);
  const enums = readFileSync(join(dir, "enums.js"), "utf8");
  const calc = readFileSync(join(dir, "calc.js"), "utf8");
  expect(enums).toContain("`[${levels.join(\", \")}] ${max == null");
  expect(calc).toContain("const names = Array.from(calc.vars.keys());");
  expect(enums + calc).not.toContain(".map((item) => item)");
  writeFileSync(join(dir, "edits.rs"), readFileSync(join(root, "test/corpus/vec_edits.rs"), "utf8").replace("fn main()", "pub fn main()"));
  run([compiler, join(dir, "edits.rs"), "-o", join(dir, "edits.js")]);
  const edits = readFileSync(join(dir, "edits.js"), "utf8");
  expect(edits).toContain("console.log(`[${v.join(\", \")}] [${w.join(\", \")}] ${asked}`);");
});

// Whether any item is one value, `any(|t| t == "hidden")`, is whether the
// array includes it, as react.dev's createFileMap asks of a meta's tokens.
// Not of floats: `includes` finds NaN, which `==` never does.
test("whether any item equals a value is whether the array includes it", async () => {
  const dir = fixture("any-equal");
  writeFileSync(join(dir, "lib.rs"), `pub fn hidden(tokens: Vec<String>, name: &str) -> (bool, bool) {
    (tokens.iter().any(|token| token == "hidden"), tokens.iter().any(|token| token == name))
}

pub fn nan(xs: Vec<f64>) -> bool {
    xs.iter().any(|&x| x == f64::NAN)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`[tokens.includes("hidden"), tokens.some((token) => token === name)]`);
  expect(js).toContain("xs.some((x) => x === NaN)");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.hidden(["a", "hidden"], "b")).toEqual([true, false]);
  expect(lib.hidden([], "b")).toEqual([false, false]);
  expect(lib.nan([NaN])).toBe(false);
});

// An iterator chain of an array MIR (ADR 0364) makes as Rust runs it, item
// by item, is the array's methods (ADR 0055) where the order its closures
// run in can't be told apart, a closure's pattern taken apart where it's
// given; where two of them do something, it stays a JS iterator's.
test("iterator chains from MIR are array methods where order can't be told", async () => {
  const dir = fixture("mir-chains");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
pub struct Crumb { pub title: String, pub hide: bool }
pub fn titles(items: &[Crumb]) -> Vec<String> {
    items.iter().enumerate().map(|(i, c)| format!("{i}:{}", c.title)).collect()
}
pub fn shown(items: &[Crumb]) -> Vec<&Crumb> {
    items.iter().filter(|c| !c.hide).collect()
}
pub fn total(items: &[u32]) -> u32 {
    items.iter().fold(0, |a, b| a + b)
}
pub fn any_big(items: &[u32]) -> bool {
    items.iter().any(|x| *x > 9)
}
// Read alone, not copied.
pub fn top(items: &[u32]) -> u32 {
    items.iter().copied().max().unwrap_or(0)
}
// Its last items, the array reversed, not copied first.
pub fn last_two(items: &[u32]) -> Vec<u32> {
    items.iter().rev().take(2).copied().collect()
}
// Each item's items, and the words of text, as arrays.
pub fn flat(groups: &[Vec<u32>]) -> Vec<u32> {
    groups.iter().flat_map(|g| g.iter().map(|x| x * 10)).collect()
}
pub fn words(s: &str) -> String {
    s.split_whitespace().map(|w| w.to_uppercase()).collect::<Vec<_>>().join(" ")
}
// Some in the middle, and a char's capital, a string already.
pub fn mid(v: &[u32]) -> Vec<u32> {
    v.iter().skip(2).take(3).copied().collect()
}
pub fn up(c: char) -> String {
    c.to_uppercase().collect()
}
// A function given as a value, its parameter named by its type.
pub fn lens(groups: &[Vec<u32>]) -> Vec<usize> {
    groups.iter().map(Vec::len).collect()
}
pub fn sizes(words: &[&str]) -> Vec<usize> {
    words.iter().copied().map(str::len).collect()
}
// What a helper makes an array of, each window, mapped as one.
pub fn diffs(v: &[i32]) -> Vec<i32> {
    v.windows(2).map(|w| w[1] - w[0]).collect()
}
// Two arrays' items side by side.
pub fn pairs(a: &[&str], b: &[u32]) -> Vec<String> {
    a.iter().zip(b.iter()).map(|(s, n)| format!("{s}{n}")).collect()
}
// Closures that only read, a string's \`find\`, or set their own
// variables, beside one that calls out, as react.dev's console filters its
// logs: arrays' methods still.
fn marked(s: &str) -> String {
    format!("<{s}>")
}
pub fn logs(lines: &[&str]) -> Vec<String> {
    lines.iter().filter(|l| l.find("error").is_none()).map(|l| marked(l)).collect()
}
pub fn signs(xs: &[i32]) -> Vec<String> {
    xs.iter()
        .map(|&x| {
            let sign;
            if x < 0 {
                sign = "-";
            } else {
                sign = "+";
            }
            sign
        })
        .map(|s| marked(s))
        .collect()
}
pub fn order(items: &[u32]) -> Vec<String> {
    let log = RefCell::new(Vec::new());
    let _ = items
        .iter()
        .map(|x| {
            log.borrow_mut().push(format!("a{x}"));
            x
        })
        .filter(|x| {
            log.borrow_mut().push(format!("b{x}"));
            true
        })
        .count();
    log.into_inner()
}
pub fn stops(items: &[u32]) -> Vec<String> {
    let log = RefCell::new(Vec::new());
    let _ = items
        .iter()
        .map(|x| {
            log.borrow_mut().push(format!("a{x}"));
            x
        })
        .any(|x| *x > 1);
    log.into_inner()
}
pub fn taken(items: &[u32]) -> Vec<String> {
    let log = RefCell::new(Vec::new());
    let _ = items
        .iter()
        .map(|x| {
            log.borrow_mut().push(format!("a{x}"));
            x
        })
        .take(1)
        .count();
    log.into_inner()
}
pub fn copied(mut items: Vec<u32>) -> (Vec<u32>, Vec<u32>) {
    let kept: Vec<u32> = items.iter().copied().collect();
    items.push(9);
    (kept, items)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  return items.map((c, i) => `${i}:${c.title}`);\n");
  expect(js).toContain("  return items.filter((c) => !c.hide);\n");
  expect(js).toContain("  return items.reduce((a, b) => (a + b) >>> 0, 0);\n");
  expect(js).toContain("  return items.some((x) => x > 9);\n");
  expect(js).toContain("  return $max(items) ?? 0;\n");
  expect(js).toContain("  return items.toReversed().slice(0, 2);\n");
  expect(js).toContain("  return groups.flatMap((g) => g.map((x) => (x * 10) >>> 0));\n");
  expect(js).toContain(".filter((word) => word !== \"\")\n    .map((w) => w.toUpperCase())\n    .join(\" \");");
  expect(js).toContain("  return v.slice(2).slice(0, 3);\n");
  expect(js).toContain("  return c.toUpperCase();\n");
  expect(js).toContain("  return groups.map((x) => x.length);\n");
  expect(js).toContain("words.map((s) => $byteLen(s))");
  expect(js).toContain("  return $windows(v, 2).map(");
  expect(js).toContain("  return $zip(a, b).map(");
  expect(js).toMatch(/return lines\s*\.filter\(/);
  expect(js).toMatch(/return xs\s*\.map\(/);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.top([2, 7, 3]), lib.top([])]).toEqual([7, 0]);
  expect([lib.lastTwo([1, 2, 3]), lib.lastTwo([1])]).toEqual([[3, 2], [1]]);
  expect([lib.flat([[1, 2], [], [3]]), lib.words(" a  b ")]).toEqual([[10, 20, 30], "A B"]);
  expect([lib.mid([1, 2, 3, 4, 5, 6]), lib.mid([1]), lib.up("ß")]).toEqual([[3, 4, 5], [], "SS"]);
  expect(lib.diffs([1, 4, 9])).toEqual([3, 5]);
  expect(lib.pairs(["x", "y", "z"], [1, 2])).toEqual(["x1", "y2"]);
  expect([lib.logs(["ok", "an error", "fine"]), lib.signs([1, -2])]).toEqual([["<ok>", "<fine>"], ["<+>", "<->"]]);
  const crumbs = [{ title: "a", hide: false }, { title: "b", hide: true }];
  expect([lib.titles(crumbs), lib.shown(crumbs), lib.total([1, 2]), lib.anyBig([3, 10])]).toEqual([["0:a", "1:b"], [crumbs[0]], 3, true]);
  expect(lib.order([1, 2])).toEqual(["a1", "b1", "a2", "b2"]);
  expect(lib.stops([1, 2, 3])).toEqual(["a1", "a2"]);
  expect(lib.taken([1, 2, 3])).toEqual(["a1"]);
  expect(lib.copied([1])).toEqual([[1], [1, 9]]);
});

// A string's parts from MIR (ADR 0364) are an array: its adapters, \`take\`
// and \`rev\` an array's; a guard returns first, the smaller branch; an
// \`if\`/\`else\` of a \`return\` in each branch is one conditional, as its
// value is; an element THIR's facts know is there is read unchecked, in a
// closure too.
test("a string's parts from MIR are an array", async () => {
  const dir = fixture("mir-split");
  writeFileSync(join(dir, "lib.rs"), `pub fn groups(group: &str) -> Vec<&str> {
    group.split(',').map(|g| g.trim()).filter(|g| !g.is_empty()).collect()
}
pub fn second(s: &str) -> Option<&str> {
    let mut parts = s.split(',');
    parts.next();
    parts.next()
}
pub fn badge(group: &str) -> Option<String> {
    let parts: Vec<&str> = group.split(',').collect();
    if parts.is_empty() {
        return None;
    }
    Some(format!("{} and {} more of {}", parts[0], parts.len() - 1, group.to_uppercase()))
}
pub fn last_two(s: &str) -> Vec<&str> {
    s.split(',').rev().take(2).collect()
}
pub fn heads(vs: &[Vec<u32>]) -> Vec<u32> {
    vs.iter().map(|v| if v.is_empty() { 0 } else { v[0] }).collect()
}
pub fn shown(group: &str) -> Option<String> {
    if group.is_empty() {
        return None;
    }
    Some(group.repeat(3).to_uppercase().trim().to_lowercase())
}
pub fn pick(c: bool, n: u32) -> u32 {
    if c {
        return n + 1;
    } else {
        return n * 2;
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  return group\n    .split(",")\n    .map((g) => $trim(g))\n    .filter(Boolean);\n');
  expect(js).toContain('  const parts = group.split(",");\n  if (parts.length === 0) {\n    return;\n  }\n  return \`\${parts[0]} and ');
  expect(js).toContain('  return s.split(",").toReversed().slice(0, 2);\n');
  expect(js).toContain("v[0]");
  // A \`return\` the program wrote stays one, the smaller branch first.
  expect(js).toContain("  if (!group) {\n    return;\n  }\n  return $trim(");
  expect(js).toContain("  if (c) {\n    return (n + 1) >>> 0;\n  } else {\n    return (n * 2) >>> 0;\n  }\n");
  expect(js).not.toContain("$index");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.groups("a, b,"), lib.second("x,y"), lib.badge("p,q"), lib.lastTwo("1,2,3"), lib.heads([[], [7]]), lib.shown("ab"), lib.pick(false, 3)]).toEqual([["a", "b"], "y", "p and 1 more of P,Q", ["3", "2"], [0, 7], "ababab", 6]);
});

// A map by what gives its value as it is, an untagged variant's constructor
// or `String::as_str`, is no map: react.dev's page route gives Next.js its
// paths, `.map(StaticPath::Params)`, and reads `NODE_ENV`.
test("a map by what gives the value itself is none", async () => {
  const dir = fixture("identity-map");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Entry {
    Name(String),
    Pair(Pair),
}
pub struct Pair {
    pub key: String,
}
pub fn entries(keys: Vec<String>) -> Vec<Entry> {
    keys.into_iter().filter(|key| !key.is_empty()).map(|key| Pair { key }).map(Entry::Pair).collect()
}
pub fn production(env: Option<&String>) -> bool {
    env.map(String::as_str) == Some("production")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).not.toContain("(value) => value");
  expect(js).toContain('return env === "production";');
  const lib = await import(join(dir, "mir.js"));
  expect(lib.entries(["a", "", "b"])).toEqual([{ key: "a" }, { key: "b" }]);
  expect([lib.production("production"), lib.production(undefined), lib.production("dev")]).toEqual([true, false, false]);
});

// ADR 0140: a generic iterator collected is `Array.from(Iterator.from(it))`,
// never `Array.from(it)`: it may be a binding's with only a `next`, of
// which `Array.from` makes an empty array.
test("a generic iterator collected keeps an iterator with only a next", async () => {
  const dir = fixture("generic-next-only");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "globalThis.counted")] fn counted() -> Box<dyn Iterator<Item = u32>> { unreachable!() }
fn plus_one<I: Iterator<Item = u32>>(items: I) -> Vec<u32> {
    items.map(|x| x + 1).collect()
}
fn listed<I: Iterator<Item = u32>>(items: I) -> Vec<u32> {
    items.collect()
}
pub fn bumped() -> Vec<u32> {
    plus_one(counted())
}
pub fn all() -> Vec<u32> {
    listed(counted())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  let n = 0;
  (globalThis as any).counted = () => ({ next: () => (n < 2 ? { value: ++n, done: false } : { value: undefined, done: true }) });
  expect(lib.bumped()).toEqual([2, 3]);
  n = 0;
  expect(lib.all()).toEqual([1, 2]);
  delete (globalThis as any).counted;
});

// ADR 0311: `.flatten()` of a read of items that may be `None` is the JS read
// itself, as react.dev's Console reads `args.shift()` and `data[0]` of any
// values: JS's `undefined` is both no item and a `None` one, as `flatten`
// makes them one.
test("flatten of a read of items that may be None is the read", async () => {
  const dir = fixture("flattened-reads");
  writeFileSync(join(dir, "lib.rs"), `use std::collections::VecDeque;

pub fn shifted(items: Vec<Option<u32>>) -> (Option<u32>, Option<u32>, usize) {
    let mut args: VecDeque<Option<u32>> = items.into_iter().collect();
    let first = args.pop_front().flatten();
    let last = args.pop_back().flatten();
    (first, last, args.len())
}

pub fn read(items: &[Option<u32>], i: usize) -> (Option<u32>, Option<u32>, Option<u32>) {
    (items.first().copied().flatten(), items.get(i).copied().flatten(), items.last().copied().flatten())
}

pub fn popped(mut items: Vec<Option<u32>>) -> Option<u32> {
    items.pop().flatten()
}
`);
  // From MIR too (ADR 0364).
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("const first = args.shift();");
  expect(js).toContain("const last = args.pop();");
  expect(js).toContain("return [items[0], items[i], items.at(-1)];");
  expect(js).toContain("return items.pop();");
  expect(js).not.toContain("$someValue");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.shifted([1, undefined, 3]), lib.shifted([undefined]), lib.shifted([])]).toEqual([[1, 3, 1], [undefined, undefined, 0], [undefined, undefined, 0]]);
  expect([lib.read([undefined, 2], 1), lib.read([], 0)]).toEqual([[undefined, 2, 2], [undefined, undefined, undefined]]);
  expect([lib.popped([1, undefined]), lib.popped([]), lib.popped([4])]).toEqual([undefined, undefined, 4]);
});

// \`v.iter().map(f).next_back()\` maps the last item alone, as Rust's
// \`Map::next_back\` does, from MIR: \`f\` runs once, and the map nothing
// steps is no statement.
test("MIR's next_back of a map runs its closure once", async () => {
  const dir = fixture("map-next-back");
  writeFileSync(join(dir, "lib.rs"), `pub fn last(v: Vec<i32>) -> (Option<i32>, u32) {
    let mut calls = 0;
    let last = v.iter().map(|x| {
        calls += 1;
        x * 10
    }).next_back();
    (last, calls)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")], 600_000);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).not.toContain(".map(");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.last([1, 2, 3]), lib.last([])]).toEqual([[30, 1], [undefined, 0]]);
});

// `fold` given a function of the crate's is `reduce` given it as it is,
// which takes no more than it's given (ADR 0279).
test("a fold given a function of the crate's gives it as it is", async () => {
  const dir = fixture("fold-crate-fn");
  writeFileSync(join(dir, "lib.rs"), `fn add(total: i32, x: &i32) -> i32 {
    total + x
}

pub fn sum(v: Vec<i32>) -> i32 {
    v.iter().fold(0, add)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain("return v.reduce(add, 0);");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.sum([1, 2, 3]), lib.sum([])]).toEqual([6, 0]);
});

// A generic iterator is stepped as JS's, `Iterator.from(items)`, as an
// array may be given (ADR 0061); the crate's own given from behind an
// `impl Iterator` is a JS iterator of it.
test("a generic iterator takes the crate's own from behind impl Iterator", async () => {
  const dir = fixture("generic-iterator-opaque");
  writeFileSync(join(dir, "lib.rs"), `pub struct Countdown {
    n: u32,
}

impl Iterator for Countdown {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.n == 0 {
            return None;
        }
        self.n -= 1;
        Some(self.n)
    }
}

pub fn countdown(n: u32) -> impl Iterator<Item = u32> {
    Countdown { n }
}

pub fn middle(items: impl Iterator<Item = u32>, k: usize) -> Vec<u32> {
    items.skip(1).take(k).collect()
}

pub fn of_countdown(n: u32) -> Vec<u32> {
    middle(countdown(n), 2)
}

pub fn mapped(n: u32) -> u32 {
    countdown(n).map(|x| x + 1).sum()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain("return Array.from(Iterator.from(items).drop(1).take(k));");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.ofCountdown(5), lib.middle([7, 8, 9, 10], 2), lib.mapped(3)]).toEqual([[3, 2], [8, 9], 6]);
});

// `rev()` of an array's items skipped and taken from, which no closure
// runs over: the array sliced, reversed.
test("rev of skipped and taken items is the slice reversed", async () => {
  const dir = fixture("rev-counted");
  writeFileSync(join(dir, "lib.rs"), `pub fn middle(v: &[i32]) -> Vec<i32> {
    v.iter().skip(1).take(2).rev().copied().collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain("return v.slice(1).slice(0, 2).toReversed();");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.middle([1, 2, 3, 4]), lib.middle([1, 2]), lib.middle([])]).toEqual([[3, 2], [2], []]);
});

test("a chain collected from MIR is a spread", async () => {
  const dir = fixture("mir-chain-collect");
  writeFileSync(join(dir, "lib.rs"), `use std::collections::VecDeque;
const MAX: usize = 2;
pub fn capped(prev: &VecDeque<u32>, more: Vec<u32>) -> VecDeque<u32> {
    let mut messages: VecDeque<u32> = prev.iter().cloned().chain(more).collect();
    while messages.len() > MAX {
        messages.pop_front();
    }
    messages
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  const messages = [...prev, ...more];\n");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.capped([1, 2], [3])).toEqual([2, 3]);
});

// Two iterators equal from MIR, \`a.eq(b)\`: each item in turn, as Rust
// steps them, till one differs or either ends.
test("iterators compared equal from MIR step both", async () => {
  const dir = fixture("mir-iter-eq");
  writeFileSync(join(dir, "lib.rs"), `pub fn palindrome(w: &str) -> bool {
    w.chars().eq(w.chars().rev())
}
pub fn same(a: &[u32], b: &[u32]) -> bool {
    a.iter().eq(b.iter())
}
pub fn differ(a: Vec<String>, b: Vec<String>) -> bool {
    a.into_iter().ne(b)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")], 600_000);
  const lib = await import(join(dir, "mir.js"));
  expect(["civic", "bob", "dog", ""].map(lib.palindrome)).toEqual([true, true, false, true]);
  expect([lib.same([1, 2], [1, 2]), lib.same([1, 2], [1]), lib.differ(["a"], ["a"]), lib.differ(["a"], ["b"])]).toEqual([true, false, false, true]);
});

// `ids.iter().position(..)` of an iterator made for it, as react.dev's
// Challenges finds the one the URL names: `$position(ids, ..)`, not of
// `$lent($iter(ids))`, which nothing else steps.
test("a position of an array's own iterator from MIR is of the array", async () => {
  const dir = fixture("position-of-array");
  writeFileSync(join(dir, "lib.rs"), `pub fn second(ids: &[String], path: &str) -> Option<usize> {
    ids.iter().position(|id| Some(id.as_str()) == path.split('#').nth(1))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('return $position(ids, (id) => id === path.split("#")[1]);');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.second(["a", "b"], "x#b"), lib.second(["a"], "x")]).toEqual([1, undefined]);
});

// A chain MIR makes lazily for one spread alone, of stages that do nothing
// else, is an array, which spreads alike, as react.dev's Console takes its
// new logs: `more.filter(..).map(..)`, not `$iter(more)`.
test("a lazy chain from MIR spread once is an array", async () => {
  const dir = fixture("chain-spread-once");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "Math.random")] fn random() -> f64 { unreachable!() }
pub fn joined(prev: &[u32], more: &[u32]) -> Vec<u32> {
    let new_logs = more.iter().filter(|&&x| x > 1).map(|&x| x * 2);
    let messages: Vec<u32> = prev.iter().cloned().chain(new_logs).collect();
    messages
}
pub fn rolled(prev: &[f64], more: &[f64]) -> usize {
    let rolled = more.iter().map(|&x| x + random());
    let all: Vec<f64> = prev.iter().cloned().chain(rolled).collect();
    all.len()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  const newLogs = more.filter((x) => x > 1).map((x) => (x * 2) >>> 0);\n");
  // Not of `Math.random()`, which does something: lazy, the array's own
  // iterator.
  expect(js).toContain("  const rolled = more.values().map((x) => x + Math.random());\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.joined([1], [1, 2, 3]), lib.rolled([1], [2])]).toEqual([[1, 4, 6], 2]);
});

// A consumer that stops early, `position`, `any`, `find`, `all`, of an
// iterator stepped again after: it takes only what Rust's takes, and what's
// left is still there for `next()`, as it is in Rust.
test("an early-stopping consumer leaves the rest of the iterator", async () => {
  const dir = fixture("early-stop-rest");
  writeFileSync(join(dir, "lib.rs"), `pub fn after(ids: &[u32]) -> Vec<Option<u32>> {
    let mut out = Vec::new();
    let mut it = ids.iter();
    out.push(it.position(|&id| id == 2).map(|i| i as u32));
    out.push(it.next().copied());
    let mut it = ids.iter();
    out.push(if it.any(|&id| id == 2) { Some(1) } else { None });
    out.push(it.next().copied());
    let mut it = ids.iter();
    out.push(it.find(|&&id| id == 2).copied());
    out.push(it.next().copied());
    let mut it = ids.iter();
    out.push(if it.all(|&id| id != 2) { Some(1) } else { None });
    out.push(it.next().copied());
    let mut it = ids.iter();
    out.push(it.find_map(|&id| (id == 2).then_some(id * 10)));
    out.push(it.next().copied());
    let mut it = ids.iter();
    out.push(it.nth(1).copied());
    out.push(it.next().copied());
    let mut it = ids.iter().peekable();
    out.push(it.peek().map(|&&id| id));
    out.push(it.position(|&id| id == 2).map(|i| i as u32));
    out.push(it.next().copied());
    out
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  expect(lib.after([1, 2, 3])).toEqual([1, 3, 1, 3, 2, 3, undefined, 3, 20, 3, 2, 3, 1, 1, 3]);
});

// A consumer of an array's own iterator, read in place only once MIR's
// temporary of it is, after an early return, `any(f)`: the array's `some`,
// as react.dev's DownloadButton checks its files.
test("an array's iterator read in place late from MIR is the array's method", async () => {
  const dir = fixture("late-eager-chain");
  writeFileSync(join(dir, "lib.rs"), `pub const SUPPORTED: [&str; 2] = ["/App.js", "/styles.css"];
pub struct Props {
    pub provided_files: Vec<String>,
}
pub fn button(Props { provided_files }: Props, supported: bool) -> Option<u32> {
    if !supported {
        return None;
    }
    if provided_files.iter().any(|file| !SUPPORTED.contains(&file.as_str())) {
        return None;
    }
    let n = provided_files.len() as u32;
    Some(n)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  if (providedFiles.some((file) => !SUPPORTED.includes(file))) {\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.button({ providedFiles: ["/App.js"] }, true), lib.button({ providedFiles: ["/x.js"] }, true), lib.button({ providedFiles: [] }, false)]).toEqual([1, undefined, undefined]);
});

// As react.dev's createFileMap reads a code block's meta:
// `!matches!(t, "pre")` is `t !== "pre"`; a string method's `bool`,
// `includes`, is one already, no `!!`; and `any(|t| t == "hidden")` of
// strings is `includes("hidden")`.
test("negated matches, string methods and any of equals from MIR read as written", async () => {
  const dir = fixture("file-map-shapes");
  writeFileSync(join(dir, "lib.rs"), `pub fn not_pre(t: &str, u: &str, out: &mut Vec<u32>) {
    if !matches!(t, "pre") && !matches!(u, "pre") {
        out.push(1);
    }
}
pub fn found(tokens: &[String]) -> Option<&String> {
    tokens.iter().find(|token| token.contains('/') || token.contains('.'))
}
pub fn hidden(tokens: &[String]) -> bool {
    tokens.iter().any(|token| token == "hidden")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  if (t !== "pre" && u !== "pre") {\n');
  expect(js).toContain('  return tokens.find((token) => token.includes("/") || token.includes("."));\n');
  expect(js).toContain('  return tokens.includes("hidden");\n');
  const lib = await import(join(dir, "mir.js"));
  const pushed: number[] = [];
  lib.notPre("a", "b", pushed);
  lib.notPre("pre", "b", pushed);
  expect([pushed, lib.found(["a", "b.js"]), lib.found(["a"]), lib.hidden(["hidden"]), lib.hidden(["x"])]).toEqual([[1], "b.js", undefined, true, false]);
});

// `count()` of an array's items, a `split`'s: its `length`, not a `reduce`
// counting each, as react.dev's CustomPreset counts a file's lines.
test("a count of an array's items from MIR is its length", async () => {
  const dir = fixture("count-length");
  writeFileSync(join(dir, "lib.rs"), `pub fn lines(code: &str) -> usize {
    code.split('\\n').count()
}
pub fn long(words: &[String]) -> usize {
    words.iter().filter(|w| w.len() > 2).count()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  return code.split("\\n").length;\n');
  expect(js).toContain("  return words.filter((w) => $byteLen(w) > 2).length;\n");
  expect(js).toContain("  return words.filter((w) => $byteLen(w) > 2).length;\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.lines("a\nb\nc"), lib.long(["a", "abc", "abcd"])]).toEqual([3, 2]);
});

// `flatten()` of `Option` items, a JS iterator of what's there, collected:
// the array's `filter`, not `Iterator.from(..)` of `$iter`'s chain, as a
// class name's parts are joined.
test("flattened options collected from MIR are the array's filter", async () => {
  const dir = fixture("flatten-collect");
  writeFileSync(join(dir, "lib.rs"), `pub fn cn(parts: &[Option<&str>]) -> String {
    parts.iter().flatten().copied().collect::<Vec<_>>().join(" ")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  return parts.filter((item) => item != null).join(" ");\n');
  const lib = await import(join(dir, "mir.js"));
  expect(lib.cn(["a", undefined, "b"])).toBe("a b");
});

// `next()` of an array's iterator that nothing else steps, a split's,
// `path.split(..).next()`, is its first item, `xs[0]`, as react.dev's
// PageHeading and NavigationBar take theirs.
test("next of a fresh iterator of an array is its first item", async () => {
  const dir = fixture("next-first");
  writeFileSync(join(dir, "lib.rs"), `pub fn clean(path: &str) -> &str {
    path.split(['?', '#']).next().unwrap_or("")
}
pub fn first(keys: Vec<String>) -> Option<String> {
    keys.into_iter().next()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('return $splitBy(path, (c) => ["?", "#"].includes(c))[0] ?? "";');
  expect(js).toContain("return keys[0];");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.clean("/a?b"), lib.clean(""), lib.first([]), lib.first(["k"])]).toEqual(["/a", "", undefined, "k"]);
});

// An iterator's adapter of a JS iterator, `.map(..)` of a range's, is a JS
// iterator already: the next adapter is its own, not `Iterator.from(..)`'s,
// nor an array's made of it, `Array.from(..).filter(..)`, as react.dev's
// useTocHighlight finds its anchors; its items collected, `Array.from(..)`.
test("an adapter of a JS iterator is not made one again", async () => {
  const dir = fixture("iterator-from-adapter");
  writeFileSync(join(dir, "lib.rs"), `pub fn evens(n: u32) -> Vec<u32> {
    (0..n).filter_map(|i| if i % 2 == 0 { Some(i * 10) } else { None }).filter(|x| *x > 5).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).not.toMatch(/Iterator\.from\(|Array\.from\([^]*Array\.from\(/);
  expect(js).toContain("  return Array.from(\n    $lazyRange(0, n)\n");
  expect(js).toContain(".filter((x) => x > 5),\n  );");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.evens(10)).toEqual([20, 40, 60, 80]);
});

// From MIR: an array's iterator made for an adapter alone, `log.iter()
// .filter(..)` of closures that do something, is the array's own,
// `log.values().filter(..)`, lazy as Rust's adapters are; no other name
// for it steps it, so no `$iter`, which knows where it is, is needed, as
// react.dev's Console filters what's logged.
test("an array's iterator for an adapter alone is its values", async () => {
  const dir = fixture("values-adapter");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "globalThis.seen")] #[allow(unused_variables)] fn seen(x: u32) -> u32 { unreachable!() }
pub fn joined(prev: &[u32], log: &[u32]) -> Vec<u32> {
    let new_logs = log.iter().filter(|x| seen(**x) != 3).map(|x| seen(x * 2));
    prev.iter().cloned().chain(new_logs).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("const newLogs = log\n    .values()\n    .filter(");
  const lib = await import(join(dir, "mir.js"));
  const order: number[] = [];
  (globalThis as any).seen = (x: number) => (order.push(x), x);
  expect(lib.joined([1], [3, 4])).toEqual([1, 8]);
  expect(order).toEqual([3, 4, 8]);
});

// A range's items each mapped, all collected, `(0..n).map(f).collect()`,
// are `$range(0, n, 4).map(f)`, an array made as big as what's collected
// anyway, room for its items of 4 bytes each asked first, as std's
// `collect` asks it (ADR 0774, amended); one filtered or taken from stays
// lazy, as a big range's array would be made for little.
test("a range mapped and collected is its array mapped", async () => {
  const dir = fixture("range-mapped");
  writeFileSync(join(dir, "lib.rs"), `pub fn squares(n: u32) -> Vec<u32> {
    (0..n).map(|i| i * i).collect()
}
pub fn few() -> Vec<u32> {
    (0..u32::MAX).filter(|i| i % 2 == 0).take(3).collect()
}
pub fn wide(n: i64) -> Vec<i64> {
    (0..n).map(|i| i * 2).collect()
}
// Its last included, backwards, or not mapped at all.
pub fn stars(n: usize) -> Vec<String> {
    (1..=n).rev().map(|k| "*".repeat(k)).collect()
}
pub fn down(n: u32) -> Vec<u32> {
    (0..n).rev().collect()
}
pub fn upto(n: u32) -> Vec<u32> {
    (1..=n).collect()
}
// Filtered, its numbers made as they're stepped, which a filter may keep
// few of: no array of them all.
pub fn odd_squares(n: u32) -> Vec<u32> {
    (0..n).filter(|i| i % 2 == 1).map(|i| i * i).collect()
}
// To a constant, included: its end the next, \`$range(1, 4)\`.
pub fn doubled() -> Vec<u32> {
    (1..=3).map(|i| i * 2).collect()
}
pub fn odd_to_five() -> Vec<u32> {
    (1..=5).filter(|i| i % 2 == 1).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("return $range(0, n, 4).map((i) => Math.imul(i, i) >>> 0);");
  expect(js).toContain('return $range(1, n + 1, 12)\n    .toReversed()\n    .map((k) => "*".repeat(k));');
  expect(js).toContain("return $range(0, n, 4).toReversed();");
  expect(js).toContain("return $range(1, n + 1, 4);");
  expect(js).toContain("return Array.from(\n    $lazyRange(0, n)\n      .filter(");
  expect(js).toContain("return $range(1, 4).map(");
  expect(js).toContain("return Array.from($lazyRange(1, 6).filter(");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.squares(4), lib.wide(3n)]).toEqual([[0, 1, 4, 9], [0n, 2n, 4n]]);
  expect([lib.stars(3), lib.stars(0), lib.down(3), lib.upto(3), lib.upto(0)]).toEqual([["***", "**", "*"], [], [2, 1, 0], [1, 2, 3], []]);
  expect(lib.oddSquares(6)).toEqual([1, 9, 25]);
  expect([lib.doubled(), lib.oddToFive()]).toEqual([[2, 4, 6], [1, 3, 5]]);
  expect(lib.few()).toEqual([0, 2, 4]);
});

// A closure that calls only a method named as a slice's that only reads,
// `at`, isn't therefore one that does nothing: the crate's own `at`,
// `P.at(p)`, may print. Its chain runs item by item, as Rust's does; a
// \`str\`'s, as Rust's types tell, is the array's own methods.
test("a crate's method named as a slice's reader still runs item by item", async () => {
  const dir = fixture("named-reader");
  writeFileSync(join(dir, "lib.rs"), `pub struct P;
impl P {
    pub fn at(&self) -> u32 {
        println!("at");
        1
    }
}
pub fn run() -> usize {
    let p = P;
    let v: Vec<u32> = vec![1, 2];
    let w: Vec<u32> = v.iter().map(|&x| x + p.at()).filter(|&x| { println!("f{x}"); true }).collect();
    w.len()
}
pub fn a_names(names: Vec<String>) -> usize {
    let w: Vec<&String> = names.iter().filter(|n| n.starts_with("a")).inspect(|n| println!("{n}")).collect();
    w.len()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  const logs: string[] = [];
  const log = console.log;
  console.log = (line: string) => logs.push(line);
  try {
    expect([lib.run(), lib.aNames(["ab", "b", "ac"])]).toEqual([2, 2]);
  } finally {
    console.log = log;
  }
  expect(logs).toEqual(["at", "f2", "at", "f3", "ab", "ac"]);
  // A \`str\`'s \`starts_with\` only reads: the array's own \`filter\`.
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain(`names\n    .filter((n) => n.startsWith("a"))\n`);
});

// A slice's `iter_mut()` is an array's items: its chain the array's own
// methods, `v.find(f)`, `$mutItems(v).filter(f)`, not a JS iterator's of
// it, `$lent(Iterator.from(v)).find(f)`.
test("iter_mut chains from MIR are array methods", async () => {
  const dir = fixture("iter-mut-chains");
  writeFileSync(join(dir, "lib.rs"), `pub fn count(v: &mut Vec<(u32, u32)>, w: u32) {
    if let Some(e) = v.iter_mut().find(|(k, _)| *k == w) {
        e.1 += 1;
    } else {
        v.push((w, 1));
    }
}
pub fn big(v: &mut [u32]) -> usize {
    v.iter_mut().filter(|x| **x > 2).count()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("const e = v.find(([k]) => k === w);");
  expect(js).toContain("return $mutItems(v).filter((x) => x.value > 2).length;");
  const lib = await import(join(dir, "mir.js"));
  const v = [[1, 1]];
  lib.count(v, 1);
  lib.count(v, 2);
  expect([v, lib.big([1, 3, 5])]).toEqual([[[1, 2], [2, 1]], 2]);
});

// A range of numbers made where it's stepped, `(0..n).find(f)`, is its
// numbers made as they're stepped from MIR, `$lazyRange(0, n).find(f)`, which
// stops where Rust's does: not an array of all of them made first, which a
// big range can't be, nor a range object stepped by `$rangeNext`. Kept lazy,
// as react.dev's getHeaderAnchors keeps a chain whose closure may panic, too.
// Of \`char\`s, a range stays one.
test("a range made where it's stepped from MIR is its numbers, made lazily", async () => {
  const dir = fixture("range-items");
  writeFileSync(join(dir, "lib.rs"), `pub fn first(n: u32, f: impl Fn(u32) -> bool) -> Option<u32> {
    (0..n).find(|&i| f(i))
}
pub fn picked(v: &[Option<u32>]) -> Vec<u32> {
    (0..v.len()).filter_map(|i| v[i]).filter(|x| x.checked_sub(1).unwrap() > 0).collect()
}
pub fn letter(f: impl Fn(char) -> bool) -> Option<char> {
    ('a'..'e').find(|&c| f(c))
}
pub fn evens(n: u32) -> Vec<u32> {
    (0..n).filter(|i| i % 2 == 0).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.first(5, (i: number) => i > 2), lib.first(2, () => false), lib.picked([3, undefined, 2])]).toEqual([3, undefined, [3, 2]]);
  expect([lib.letter((c: string) => c > "b"), lib.letter(() => false)]).toEqual(["c", undefined]);
  expect(lib.evens(5)).toEqual([0, 2, 4]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  // Of a filter, which may keep few of many, no array of them all either.
  expect(js).toContain("return Array.from($lazyRange(0, n).filter((i) => i % 2 === 0));");
  expect(js).toContain("return $lazyRange(0, n).find(f);");
  expect(js).toContain("return Array.from(\n    $lazyRange(0, v.length)\n      .map(");
  // A billion numbers, the first that holds the third: as quick as Rust's.
  const { first } = await import(join(dir, "mir.js"));
  expect(first(1_000_000_000, (i: number) => i === 2)).toBe(2);
});

// A range cloned and stepped is the copy stepped: the range itself, read
// after, is as it was, `1..4`. Found in review: MIR stepped the range
// itself, `$iterator(r, $rangeNext)`, which `4..4` after.
test("a range cloned and stepped leaves the range as it was", async () => {
  const dir = fixture("range-clone-stepped");
  writeFileSync(join(dir, "lib.rs"), `pub fn counted(a: i32, z: i32) -> String {
    let r = a..z;
    let c = r.clone().count();
    format!("{c} {:?}", r)
}
pub fn summed(a: i32, z: i32) -> String {
    let r = a..=z;
    let s: i32 = r.clone().sum();
    format!("{s} {:?}", r)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.counted(1, 4), lib.summed(1, 3)]).toEqual(["3 1..4", "6 1..=3"]);
});

// An iterator of the crate's `len()`: std's of its `size_hint()`, which
// panics where that isn't exact, or its own where it has one.
test("an iterator's len from MIR is its own or its size hint's", async () => {
  const dir = fixture("iterator-len");
  writeFileSync(join(dir, "lib.rs"), `pub struct Span { pub low: u32, pub high: u32 }
impl Iterator for Span {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.low < self.high { self.low += 1; Some(self.low - 1) } else { None }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.high - self.low) as usize;
        (n, Some(n))
    }
}
impl ExactSizeIterator for Span {}
pub struct Own;
impl Iterator for Own {
    type Item = u8;
    fn next(&mut self) -> Option<u8> { None }
}
impl ExactSizeIterator for Own {
    fn len(&self) -> usize { 42 }
}
pub fn lens() -> (usize, usize) {
    let mut s = Span { low: 0, high: 5 };
    s.next();
    (s.len(), Own.len())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  expect((await import(join(dir, "mir.js"))).lens()).toEqual([4, 42]);
});

// An iterator of the crate's from MIR: what its impl writes itself, its
// own `count`, is its; what it takes from `Iterator`, `map`, `fuse` or
// `collect`, std's over its own `next`; and given to another's `chain`, its
// items, not its fields.
test("the crate's iterator from MIR: its own methods, std's, and given on", async () => {
  const dir = fixture("user-iterator-methods");
  writeFileSync(join(dir, "lib.rs"), `pub struct Counter(pub u32);
impl Iterator for Counter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 < 3 { self.0 += 1; Some(self.0) } else { None }
    }
    fn count(self) -> usize { 99 }
}
pub fn run() -> (usize, Vec<u32>, Vec<u32>) {
    let own = Counter(0).count();
    let mapped: Vec<u32> = Counter(0).map(|n| n * 10).fuse().collect();
    let chained: Vec<u32> = [7].into_iter().chain(Counter(1)).collect();
    (own, mapped, chained)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  expect((await import(join(dir, "mir.js"))).run()).toEqual([99, [10, 20, 30], [7, 2, 3]]);
});

// The crate's iterator stepped through a `&mut` to it, `by_ref()`, from
// MIR: std's `next` of a `&mut I` is `I`'s own, and what's left is still
// there after.
test("the crate's iterator stepped through by_ref from MIR", async () => {
  const dir = fixture("user-iterator-by-ref");
  writeFileSync(join(dir, "lib.rs"), `pub struct Counter(pub u32);
impl Iterator for Counter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 < 5 { self.0 += 1; Some(self.0) } else { None }
    }
}
pub fn run() -> (Vec<u32>, Vec<u32>) {
    let mut all = Counter(0);
    let mut first = Vec::new();
    for n in all.by_ref() {
        first.push(n);
        if n == 2 { break; }
    }
    let rest: Vec<u32> = all.collect();
    (first, rest)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  expect((await import(join(dir, "mir.js"))).run()).toEqual([[1, 2], [3, 4, 5]]);
});

// A `char` range stepped as Rust steps it, by its code point, past the
// surrogates no `char` is: in a `for`, given to `extend`, and an iterator
// turned around after it's been stepped from both ends.
test("char ranges step by code point, and a stepped iterator turns around", async () => {
  const dir = fixture("char-ranges");
  writeFileSync(join(dir, "lib.rs"), `pub fn letters() -> String {
    let mut s = String::new();
    for c in 'a'..'e' {
        s.push(c);
    }
    for c in 'x'..='z' {
        s.push(c);
    }
    s
}
pub fn from(a: char, b: char) -> String {
    let mut s = String::new();
    for c in a..b {
        s.push(c);
    }
    s
}
pub fn across() -> Vec<u32> {
    let mut out = Vec::new();
    for c in '\\u{d7fe}'..'\\u{e001}' {
        out.push(c as u32);
    }
    out
}
pub fn extended() -> Vec<u32> {
    let mut v: Vec<char> = vec!['a'];
    v.extend('b'..'d');
    v.extend('\\u{d7fe}'..'\\u{e001}');
    v.into_iter().map(|c| c as u32).collect()
}
pub fn counted(n: u32) -> u32 {
    let mut total = 0;
    for i in 0..n {
        total += i;
    }
    total
}
pub fn between(k: u32, n: u32) -> u32 {
    let mut total = 0;
    for i in k..n {
        total += i;
    }
    total
}
pub fn reversed() -> Vec<u32> {
    let mut it = [1u32, 2, 3, 4].iter();
    it.next_back();
    it.next();
    it.rev().copied().collect()
}
pub fn kept_reversed(v: &[u32]) -> Vec<u32> {
    let it = v.iter();
    it.rev().copied().collect()
}
pub fn owned_reversed(v: Vec<u32>) -> Vec<u32> {
    let mut it = v.into_iter();
    it.next();
    it.rev().collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await import(join(dir, "lib.js"));
  expect(lib.letters()).toBe("abcdxyz");
  expect(lib.from("p", "t")).toBe("pqrs");
  expect(lib.across()).toEqual([0xd7fe, 0xd7ff, 0xe000]);
  expect(lib.extended()).toEqual([97, 98, 99, 0xd7fe, 0xd7ff, 0xe000]);
  expect(lib.counted(5)).toBe(10);
  expect(lib.reversed()).toEqual([3, 2]);
  expect([lib.keptReversed([1, 2, 3]), lib.ownedReversed([1, 2, 3])]).toEqual([[3, 2, 1], [3, 2]]);
  // A number's range is still JS's counted `for`.
  expect(lib.between(2, 5)).toBe(9);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("for (let i = 0; i < n; i++) {");
  expect(js).toContain("for (let i = k; i < n; i++) {");
});

// ADR 0139 amended: `zip` is `$lazyZip`, its first side's item taken
// before its second's, as `Zip::next` does: one more of `a`'s where `b`
// ends first. Of a side that isn't `TrustedLen`, a filter's, as std folds
// it too; of two that are, std's struct (ADR 0997).
test("zip takes its first side's item before its second's", () => {
  const dir = fixture("zip-first-side");
  writeFileSync(
    join(dir, "lib.rs"),
    `pub fn main() {
    let v = [1, 2, 3];
    let z: Vec<_> = v.iter().filter(|x| { println!("a{x}"); true }).zip([10, 20]).collect();
    println!("{z:?}");
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("$lazyZip(");
  const zip = readFileSync(join(root, "src/runtime/lazy_zip.js"), "utf8");
  expect(zip).toContain("const item = left.next();\n    if (item.done) return item;\n    const other = right.next();");
});

// ADR 0748: a `for` of a slice's first items, or of those past some,
// `xs.iter().take(n)`, `xs.iter().skip(n)`, is of the array's part,
// `xs.slice(0, n)`, `xs.slice(n)`, as a chain's `take` is; of their
// `enumerate().take(n)`, the part's `entries()`. Their `enumerate().skip(n)`
// keeps each index, `xs.entries().drop(n)`. React's ported getArrayKind
// steps `array.slice(0, OBJECT_WIDTH_LIMIT)`.
test("a for of a slice's first items is of its part", async () => {
  const dir = fixture("for-of-taken");
  writeFileSync(join(dir, "lib.rs"), `const LIMIT: usize = 3;

pub fn total(xs: &[u32]) -> u32 {
    let mut sum = 0;
    for x in xs.iter().take(LIMIT) {
        sum += x;
    }
    sum
}

pub fn rest(xs: &[u32]) -> u32 {
    let mut sum = 0;
    for &x in xs.iter().skip(1) {
        sum += x;
    }
    sum
}

pub fn named(xs: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for (i, x) in xs.iter().enumerate().take(LIMIT) {
        out.push(format!("{i}{x}"));
    }
    out
}

pub fn named_past(xs: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for (i, x) in xs.iter().enumerate().skip(1) {
        out.push(format!("{i}{x}"));
    }
    out
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("for (const x of xs.slice(0, LIMIT)) {");
  expect(js).toContain("for (const x of xs.slice(1)) {");
  expect(js).toContain("for (const [i, x] of xs.slice(0, LIMIT).entries()) {");
  expect(js).toContain("for (const [i, x] of xs.entries().drop(1)) {");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.total([1, 2, 3, 4]), lib.total([5])]).toEqual([6, 5]);
  expect(lib.rest([1, 2, 3])).toBe(5);
  expect(lib.named(["a", "b", "c", "d"])).toEqual(["0a", "1b", "2c"]);
  expect(lib.namedPast(["a", "b", "c"])).toEqual(["1b", "2c"]);
  const of = performanceTrack();
  expect(of("getArrayKind")).toContain("for (const value of array.slice(0, OBJECT_WIDTH_LIMIT)) {");
  expect(of("addValueToProperties")).toContain("for (const entry of value.slice(0, OBJECT_WIDTH_LIMIT)) {");
});

// ADR 0480: a clone of a `copied()` is a clone of the iterator it copies,
// its `Clone` dictionary's where it's generic: each goes on apart from
// where it was.
test("a clone of a generic iterator's copied() is its clone", async () => {
  const dir = fixture("copied-clone");
  writeFileSync(join(dir, "lib.rs"), `fn apart<'a, I: Iterator<Item = &'a u32> + Clone>(it: I) -> (u32, u32) {
    let mut copied = it.copied();
    copied.next();
    let again = copied.clone();
    (copied.sum(), again.max().unwrap_or(0))
}

pub fn each(v: Vec<u32>) -> [(u32, u32); 3] {
    [apart(v.iter()), apart(v.iter().rev()), apart(v.iter().skip(1))]
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("  const again = $givenClone(copied, IClone.clone);\n");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.each([1, 2, 3])).toEqual([[5, 3], [3, 2], [3, 3]]);
});
