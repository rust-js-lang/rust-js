// What the compiler writes of generic code's iterators: one given as JS's,
// `Iterator.from(items)`, collected or folded, the crate's own given from
// behind an `impl Iterator`, a clone of a `copied()`, and a trait's
// associated iterators an impl's dictionary makes JS iterators. Split from
// lowering-iterators.test.ts by concept, as a test file is at most 1500
// lines. Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildCompiler, compiler, fixture, run } from "./support";

beforeAll(buildCompiler, 600_000);

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
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain("return Array.from($roomForHint(Iterator.from(items).drop(1).take(k), 4, false));");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.ofCountdown(5), lib.middle([7, 8, 9, 10], 2), lib.mapped(3)]).toEqual([[3, 2], [8, 9], 6]);
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

// #182: what generic code collects by `fold` of a `skip` is `$folded`, which
// takes the items by the crate's own fold where its iterator has one; a
// JS iterator with none, an array's, a str's chars', is stepped as it is.
test("a fold-taking collect in generic code of a JS iterator steps it", async () => {
  const dir = fixture("folded-of-js-iterators");
  writeFileSync(
    join(dir, "lib.rs"),
    `fn skipped<I: Iterator<Item = char>>(it: I) -> String {
    it.skip(1).collect()
}

pub fn of_vec(letters: Vec<char>) -> String {
    skipped(letters.into_iter())
}

pub fn of_str(text: &str) -> String {
    skipped(text.chars())
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('  return Array.from($reserveHint($folded(Iterator.from(it).drop(1)), 1)).join("");\n');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.ofVec(["a", "b", "c"]), lib.ofStr("xyz"), lib.ofStr("")]).toEqual(["bc", "yz", ""]);
});

// A trait's associated iterator, and what its method hides behind an
// `impl`, that an impl gives as std's struct of an adapter: generic code's
// flatten of them, stepped from its back, steps each by its compiled
// `next_back`, as the impl's dictionary makes it a JS iterator that knows
// it, where it was stepped as an empty one; native Rust prints these
// lines (#1292). The build of std's structs too, which asks the flatten's
// constant size of the trait's dictionaries, `IterConstSize` and
// `HiddenConstSize`, where it refused it (#1312, #1341).
test("a flatten of a trait's associated iterators steps each from its back as the impl's dictionary makes it", async () => {
  const dir = fixture("associated-iterator-flatten-back");
  writeFileSync(
    join(dir, "lib.rs"),
    `trait Source {
    type Iter: DoubleEndedIterator<Item = u32>;
    fn items(n: u32) -> Self::Iter;
    fn hidden(n: u32) -> impl DoubleEndedIterator<Item = u32>;
}

struct Tens;

impl Source for Tens {
    type Iter = std::iter::Map<std::ops::Range<u32>, fn(u32) -> u32>;
    fn items(n: u32) -> Self::Iter {
        (0..n).map(|x| x * 10)
    }
    fn hidden(n: u32) -> impl DoubleEndedIterator<Item = u32> {
        (0..n).map(|x| x + 1)
    }
}

fn by_name<S: Source>() -> Vec<u32> {
    (1..4u32).flat_map(S::items).rev().collect()
}

fn hidden<S: Source>() -> Vec<u32> {
    (1..4u32).flat_map(|n| S::hidden(n)).rev().collect()
}

pub fn flattened() -> Vec<Vec<u32>> {
    vec![by_name::<Tens>(), hidden::<Tens>()]
}
`,
  );
  for (const build of [[], ["--std-iterators"]]) {
    const out = join(dir, build.length === 0 ? "lib.js" : "structs.js");
    run([compiler, join(dir, "lib.rs"), "-o", out, ...build]);
    const lib = await import(out);
    expect(lib.flattened()).toEqual([
      [20, 10, 0, 10, 0, 0],
      [3, 2, 1, 2, 1, 1],
    ]);
  }
});

// Of a trait method with generics of its own, a type's, `items<U: Copy>`,
// or a constant's, `hidden<const M: u32>`: generic code's flatten of what
// each gives, stepped from its back, steps each by its compiled
// `next_back`, as the impl's dictionary makes it, where `$iter` threw that
// it doesn't know its back; native Rust prints these lines (#1308). The
// build of std's structs too, which asks the flatten's constant size of
// `hidden`'s `impl Trait` of the trait's dictionaries, `HiddenConstSize`,
// where it refused it (#1312).
test("a flatten of what a trait method with its own generics gives steps each from its back", async () => {
  const dir = fixture("associated-iterator-own-generics-flatten");
  writeFileSync(
    join(dir, "lib.rs"),
    `trait Source {
    type Iter: DoubleEndedIterator<Item = u32>;
    fn items<U: Copy>(n: u32, tag: U) -> Self::Iter;
    fn hidden<const M: u32>(n: u32) -> impl DoubleEndedIterator<Item = u32>;
}

struct Tens;

impl Source for Tens {
    type Iter = std::iter::Map<std::ops::Range<u32>, fn(u32) -> u32>;
    fn items<U: Copy>(n: u32, _tag: U) -> Self::Iter {
        (0..n).map(|x| x * 10)
    }
    fn hidden<const M: u32>(n: u32) -> impl DoubleEndedIterator<Item = u32> {
        (0..n).map(|x| x * M)
    }
}

fn by_name<S: Source>() -> Vec<u32> {
    (1..4u32).flat_map(|n| S::items(n, 'a')).rev().collect()
}

fn hidden<S: Source>() -> Vec<u32> {
    (1..4u32).flat_map(S::hidden::<2>).rev().collect()
}

pub fn flattened() -> Vec<Vec<u32>> {
    vec![by_name::<Tens>(), hidden::<Tens>()]
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await import(join(dir, "lib.js"));
  expect(lib.flattened()).toEqual([
    [20, 10, 0, 10, 0, 0],
    [4, 2, 0, 2, 0, 0],
  ]);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "structs.js"), "--std-iterators"]);
  expect(readFileSync(join(dir, "structs.js"), "utf8")).toContain("SSource.HiddenConstSize");
  const structs = await import(join(dir, "structs.js"));
  expect(structs.flattened()).toEqual([
    [20, 10, 0, 10, 0, 0],
    [4, 2, 0, 2, 0, 0],
  ]);
});

// `$iter` of what's neither an array nor an iterator that knows its back,
// a compiler's mistake, throws, never steps it as an empty array, as
// #1292's flatten did of std's struct; a `TypeError`, which no
// `catch_unwind` takes for a panic.
test("an iterator that doesn't know its back is never stepped as an empty array", async () => {
  // The runtime a compiled module imports, a mutant's too.
  const dir = fixture("iter-of-what-knows-no-back");
  writeFileSync(join(dir, "lib.rs"), "pub fn none() {}\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  writeFileSync(join(dir, "iter.js"), 'export { $iter } from "@rust-js/std";\n');
  const { $iter } = await import(join(dir, "iter.js"));
  expect(() => $iter({ next: () => ({ done: true }) })).toThrow(TypeError);
  expect([...$iter([1, 2])]).toEqual([1, 2]);
});

// A generic collect of items with a destructor asks its caller their
// in-place layout only where its lowering reads it, a \`Vec\` it collects
// itself: not one a \`B: FromIterator\` bound's dictionary collects, nor one
// into a \`LinkedList\`, which std extends as each item comes (#1443).
test("a generic collect asks no in-place layout it doesn't read", () => {
  const dir = fixture("collect-in-place-unread");
  writeFileSync(join(dir, "lib.rs"), `use std::collections::LinkedList;

pub struct Loud(pub u32);

impl Drop for Loud {
    fn drop(&mut self) {}
}

pub fn gathered<I: Iterator, B: FromIterator<I::Item>>(it: I) -> B {
    it.collect()
}

pub fn listed<I: Iterator>(it: I) -> LinkedList<I::Item> {
    it.collect()
}

pub fn both() -> usize {
    let v: Vec<Loud> = gathered(vec![Loud(1)].into_iter());
    v.len() + listed(vec![Loud(2)].into_iter()).len()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function gathered(it, BFromIteratorIItem, ITrustedLen, IInPlace, dropI) {");
  expect(js).toContain("export function listed(it, dropI, dropIItem) {");
});
