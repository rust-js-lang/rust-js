// What the compiler writes of a `Vec`'s capacity in a crate that reads one,
// where std's growth of it isn't kept (#1180): `capacity()` throws a refusal
// naming #1180, never another number. Each test is a program of its own,
// compiled and run.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

// A `Vec`'s capacity std's growth of it wasn't kept of, in a crate that
// reads one: one std collects in place, whose capacity is its source's,
// where that source is no variable the chain moves nor found through
// generic code's adapters (#1259), and a string's bytes, whose capacity is the string's. `capacity()` of it
// throws a refusal naming #1180, `$unsupported`'s, never another number,
// which no `catch_unwind` takes for a panic (#1193); `shrink_to_fit()` knows it again, and a `push`
// past it leaves it unknown (the user's decision, #1180).
test("capacity() of a Vec whose growth wasn't kept throws, naming #1180", async () => {
  const dir = fixture("capacity-unknown");
  writeFileSync(
    join(dir, "lib.rs"),
    `fn grown() -> Vec<u32> {
    let mut v = Vec::new();
    for i in 0..5 {
        v.push(i);
    }
    v
}

pub fn in_place() -> usize {
    let w: Vec<u32> = grown().into_iter().map(|x| x + 1).collect();
    w.capacity()
}

fn collected<I: Iterator, B: FromIterator<I::Item>>(it: I) -> B {
    it.collect()
}

pub fn generic_in_place() -> usize {
    let w: Vec<u32> = collected(grown().into_iter().map(|x| x + 1));
    w.capacity()
}

pub fn bytes() -> usize {
    String::from("abc").into_bytes().capacity()
}

pub fn caught() -> bool {
    std::panic::catch_unwind(|| {
        let w: Vec<u32> = grown().into_iter().map(|x| x + 1).collect();
        w.capacity()
    })
    .is_err()
}

pub fn fitted() -> usize {
    let mut w: Vec<u32> = grown().into_iter().map(|x| x + 1).collect();
    w.shrink_to_fit();
    w.capacity()
}

pub fn pushed() -> usize {
    let mut w: Vec<u32> = grown().into_iter().map(|x| x + 1).collect();
    w.push(9);
    w.capacity()
}

pub fn known() -> usize {
    grown().capacity()
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await import(join(dir, "lib.js"));
  for (const f of [lib.inPlace, lib.bytes, lib.pushed, lib.caught]) {
    expect(() => f()).toThrow("rust-js does not support `capacity()` of a `Vec` whose growth it didn't keep");
  }
  // Generic code's in-place collect finds its source `Vec` through the
  // adapters it steps, so its capacity is its source's buffer's, std's 8
  // (#1259); a collect whose source is a call's `Vec`, no variable, isn't.
  expect([lib.fitted(), lib.known(), lib.genericInPlace()]).toEqual([5, 8, 8]);
});

// A `Vec` generic code grows by an iterator its caller gives, whose hint
// std grows it by: where one caller's JS hint isn't std's, a string's
// `chars()`, or a `skip(n)` JS makes of its own, its capacity is unknown
// once it grows, and `capacity()` throws, naming #1180, as a `Vec` grown
// by that iterator where its type is known is refused (#1180's (3)). One
// whose every caller's hint is std's, a range's or a filter's, keeps std's,
// as does one it never grew. One JS function is every caller's, so where
// one caller's hint isn't std's, each caller's growth there is unknown. A
// `pub` one's, whose callers aren't all seen, is asked of each value it's
// given: unknown where its hint meets JS's own iterator, which knows none,
// a string's `chars().skip(n)`'s `drop(n)`, kept of rust-js's own (#1251).
test("capacity() of a Vec generic code grew by a hint JS lost throws, naming #1180", async () => {
  const dir = fixture("capacity-generic-hint-lost");
  writeFileSync(
    join(dir, "lib.rs"),
    `fn grow<I: Iterator<Item = char>>(v: &mut Vec<char>, it: I) {
    v.extend(it);
}

fn grow_skipped<I: Iterator<Item = char>>(v: &mut Vec<char>, it: I) {
    v.extend(it.skip(2));
}

fn grow_nums<I: Iterator<Item = u32>>(v: &mut Vec<u32>, it: I) {
    v.extend(it);
}

fn grow_counted<I: Iterator<Item = u32>>(v: &mut Vec<u32>, it: I) {
    v.extend(it);
}

struct Countdown(u32);

impl Iterator for Countdown {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some(self.0)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0 as usize, Some(self.0 as usize))
    }
}

impl ExactSizeIterator for Countdown {}

fn grow_own<I: Iterator<Item = u32>>(v: &mut Vec<u32>, it: I) {
    v.extend(it);
}

pub fn own_mapped() -> usize {
    let mut v = Vec::new();
    grow_own(&mut v, Countdown(5).map(|n| n * 2));
    v.capacity()
}

pub fn grow_any<I: Iterator<Item = char>>(v: &mut Vec<char>, it: I) {
    v.extend(it);
}

const TEXT: &str = "the quick brown fox jumps over the lazy dog";

struct Odds(u32);

impl Iterator for Odds {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        self.0 += 2;
        (self.0 < 40).then_some('o')
    }
}

pub fn any_own() -> usize {
    let mut v = Vec::new();
    grow_any(&mut v, Odds(0));
    v.capacity()
}

pub fn any_skipped() -> usize {
    let mut v = Vec::new();
    grow_any(&mut v, TEXT.chars().skip(5));
    v.capacity()
}

pub fn any_counted() -> usize {
    let mut v = Vec::new();
    grow_any(&mut v, ('a'..='z').filter(|c| *c != 'q'));
    v.capacity()
}

pub fn chars_skipped() -> usize {
    let mut v = Vec::new();
    grow(&mut v, TEXT.chars().skip(5));
    v.capacity()
}

pub fn skipped_here() -> usize {
    let mut v = vec!['x'];
    grow_skipped(&mut v, TEXT.chars());
    v.capacity()
}

pub fn vec_skipped() -> usize {
    let mut v = vec![1];
    grow_nums(&mut v, vec![1u32, 2, 3, 4, 5].into_iter().skip(1));
    v.capacity()
}

pub fn range() -> usize {
    let mut v = Vec::new();
    grow_counted(&mut v, 0..40);
    v.capacity()
}

pub fn filtered() -> usize {
    let mut v = Vec::new();
    grow_counted(&mut v, (0..40).filter(|n| n % 3 == 0));
    v.capacity()
}

pub fn roomy() -> usize {
    let mut v = Vec::with_capacity(100);
    grow(&mut v, TEXT.chars().skip(5));
    v.capacity()
}

pub fn never_grown() -> usize {
    let mut v = vec!['x'];
    grow_skipped(&mut v, "ab".chars());
    v.capacity()
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await import(join(dir, "lib.js"));
  for (const f of [lib.charsSkipped, lib.skippedHere, lib.vecSkipped, lib.anySkipped]) {
    expect(() => f()).toThrow("rust-js does not support `capacity()` of a `Vec` whose growth it didn't keep");
  }
  expect([lib.range(), lib.filtered(), lib.roomy(), lib.neverGrown(), lib.anyCounted(), lib.ownMapped(), lib.anyOwn()]).toEqual([40, 16, 100, 1, 32, 5, 32]);
});

// A `collect()` std makes in place of a chain an `impl Iterator` function
// gives, which no variable of the body is the source of: in the std build,
// std's structs link each adapter to its source, walked to the `Vec` it
// collects in, so its capacity is std's, `src_cap * src_size / dst_size`;
// in the plain build, JS's own helpers keep no link, so `capacity()` refuses
// it, naming #1180, never another number (#1428, the user's decision:
// won't-fix, the readability of giving them links isn't worth a capacity).
test("an in-place collect through an impl Iterator function has std's capacity in the std build", async () => {
  const dir = fixture("capacity-in-place-opaque");
  writeFileSync(join(dir, "lib.rs"), `fn evens(v: Vec<u32>) -> impl Iterator<Item = u32> {
    v.into_iter().filter(|x| x % 2 == 0)
}

fn passed<I: Iterator>(it: I) -> impl Iterator<Item = I::Item> {
    it
}

pub fn through_a_function() -> usize {
    let mut v: Vec<u32> = Vec::with_capacity(9);
    v.extend([1, 2, 3, 4]);
    let halves: Vec<u32> = evens(v).map(|x| x / 2).collect();
    halves.capacity()
}

pub fn through_an_identity() -> usize {
    let mut v: Vec<[u32; 2]> = Vec::with_capacity(3);
    v.extend([[1, 2], [3, 4]]);
    let sums: Vec<u32> = passed(v.into_iter()).map(|[a, b]| a + b).collect();
    sums.capacity()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "structs.js"), "--std-iterators"]);
  const structs = await import(join(dir, "structs.js"));
  expect([structs.throughAFunction(), structs.throughAnIdentity()]).toEqual([9, 6]);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await import(join(dir, "lib.js"));
  for (const f of [lib.throughAFunction, lib.throughAnIdentity]) {
    expect(() => f()).toThrow("rust-js does not support `capacity()` of a `Vec` whose growth it didn't keep");
  }
});

// #934's case: an impl's method growing a \`Vec\` of an associated type's
// items, \`I::Item\`, whose size its dictionary can't give, compiles, its
// size \`undefined\` (\`room_value\`): what it grows runs as native Rust does,
// its capacity isn't known, and \`capacity()\` refuses it, naming #1180, never
// another number, of a fresh \`Vec\` too, whose items may be of no size, as
// \`()\`'s, whose capacity is \`usize::MAX\` (#1260, the user's decision).
test("an impl's growth of an associated type's items compiles, its capacity refused", async () => {
  const dir = fixture("capacity-impl-associated-items");
  writeFileSync(join(dir, "lib.rs"), `pub struct Buf<I: Iterator> {
    it: I,
}

pub trait Fill {
    fn filled(&mut self) -> usize;
    fn room(&mut self) -> usize;
    fn fresh(&self) -> usize;
    fn pushed(&mut self) -> usize;
    fn reserved(&self) -> usize;
}

impl<I: Iterator> Fill for Buf<I> {
    fn filled(&mut self) -> usize {
        let mut v: Vec<I::Item> = Vec::new();
        for x in &mut self.it {
            v.push(x);
        }
        v.len()
    }
    fn room(&mut self) -> usize {
        let mut v: Vec<I::Item> = Vec::new();
        v.extend(&mut self.it);
        v.capacity()
    }
    fn fresh(&self) -> usize {
        let v: Vec<I::Item> = Vec::new();
        v.capacity()
    }
    fn pushed(&mut self) -> usize {
        let mut v: Vec<I::Item> = Vec::new();
        if let Some(x) = self.it.next() {
            v.push(x);
        }
        v.capacity()
    }
    fn reserved(&self) -> usize {
        let mut v: Vec<I::Item> = Vec::new();
        v.reserve(3);
        v.capacity()
    }
}

pub fn filled() -> usize {
    let mut b: Box<dyn Fill> = Box::new(Buf { it: [1u8, 2, 3].into_iter() });
    b.filled()
}

pub fn room() -> usize {
    let mut b: Box<dyn Fill> = Box::new(Buf { it: [1u8, 2, 3].into_iter() });
    b.room()
}

pub fn pushed() -> usize {
    let mut b: Box<dyn Fill> = Box::new(Buf { it: [1u8, 2, 3].into_iter() });
    b.pushed()
}

pub fn reserved() -> usize {
    let b: Box<dyn Fill> = Box::new(Buf { it: [1u8, 2, 3].into_iter() });
    b.reserved()
}

pub fn fresh_units() -> usize {
    let b: Box<dyn Fill> = Box::new(Buf { it: [(), ()].into_iter() });
    b.fresh()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await import(join(dir, "lib.js"));
  expect(lib.filled()).toBe(3);
  for (const f of [lib.room, lib.freshUnits, lib.pushed, lib.reserved]) expect(() => f()).toThrow("rust-js does not support");
  expect(() => lib.freshUnits()).toThrow("`capacity()` of a `Vec` whose growth it didn't keep");
});

// A slot asked both its size, \`size_of\`'s, and the size std grows a \`Vec\`
// by, \`TypeFact::Room\`, named alike, is given once, \`TSize\`, never twice
// (\`settle_room\`, #1260).
test("a type parameter's size and its growth's are one TSize", () => {
  const dir = fixture("capacity-size-and-room");
  writeFileSync(join(dir, "lib.rs"), `pub fn sized_and_grown<T: Copy>(x: T) -> (usize, usize) {
    let mut v: Vec<T> = Vec::new();
    v.push(x);
    (std::mem::size_of::<T>(), v.capacity())
}

pub fn both() -> (usize, usize) {
    sized_and_grown(1u8)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("function sizedAndGrown(x, TSize) {");
});

// What an impl grew of its associated type's items without their size,
// #934's, returned to a caller that knows it, \`Vec<u8>\`: its capacity still
// isn't known, so \`capacity()\` refuses it, never the number growth by no
// size gives (#1260).
test("a Vec an impl grew without its items' size has no capacity its caller reads", async () => {
  const dir = fixture("capacity-impl-grown-returned");
  writeFileSync(join(dir, "lib.rs"), `pub struct Buf<I: Iterator> {
    it: I,
}

pub trait Fill {
    type Out;
    fn pushed(&mut self) -> Vec<Self::Out>;
    fn reserved(&mut self) -> Vec<Self::Out>;
}

impl<I: Iterator> Fill for Buf<I> {
    type Out = I::Item;
    fn pushed(&mut self) -> Vec<I::Item> {
        let mut v = Vec::new();
        if let Some(x) = self.it.next() {
            v.push(x);
        }
        v
    }
    fn reserved(&mut self) -> Vec<I::Item> {
        let mut v = Vec::new();
        v.reserve(3);
        v
    }
}

pub fn pushed() -> usize {
    let mut b = Buf { it: [1u8, 2].into_iter() };
    b.pushed().capacity()
}

pub fn reserved() -> usize {
    let mut b = Buf { it: [1u8, 2].into_iter() };
    b.reserved().capacity()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await import(join(dir, "lib.js"));
  for (const f of [lib.pushed, lib.reserved]) expect(() => f()).toThrow("`capacity()` of a `Vec` whose growth it didn't keep");
});

// What an impl's method gives a function it calls of its associated type's
// items' size, #934's \`undefined\`: that function's \`extend\` of its \`Vec\`
// grows as one of items whose size no caller gives, asking no hint, its
// capacity then not known: itertools' \`LazyBuffer::prefill\` of its
// \`Combinations\`' \`next\` (#1260).
test("an extend given its items' size as undefined runs as one of unsized items", async () => {
  const dir = fixture("capacity-undefined-size-extend");
  writeFileSync(join(dir, "lib.rs"), `pub struct Buffer<I: Iterator> {
    it: I,
    buffer: Vec<I::Item>,
}

impl<I: Iterator> Buffer<I> {
    fn prefill(&mut self, len: usize) {
        if len > self.buffer.len() {
            let delta = len - self.buffer.len();
            self.buffer.extend(self.it.by_ref().take(delta));
        }
    }
}

pub struct Pairs<I: Iterator> {
    buffer: Buffer<I>,
}

impl<I: Iterator> Iterator for Pairs<I>
where
    I::Item: Copy,
{
    type Item = usize;
    fn next(&mut self) -> Option<usize> {
        self.buffer.prefill(self.buffer.buffer.len() + 2);
        Some(self.buffer.buffer.len())
    }
}

pub fn lengths() -> Vec<usize> {
    let mut pairs = Pairs { buffer: Buffer { it: 0u8..20, buffer: Vec::new() } };
    vec![pairs.next().unwrap(), pairs.next().unwrap(), pairs.next().unwrap()]
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await import(join(dir, "lib.js"));
  expect(lib.lengths()).toEqual([2, 4, 6]);
});

// A \`reserve\` of a trait's default's \`Vec<Option<Self::Item>>\` an impl
// copies, a built type of its associated type: its size given by the
// impl's dictionary's maker, its capacity std's 4, never 3 as if its items
// had no size (#1432).
test("a copied default's reserve of a built type of its item has std's capacity", async () => {
  const dir = fixture("capacity-reserve-unsized");
  writeFileSync(join(dir, "lib.rs"), `pub struct Wrapper<I>(I);

impl<I: Iterator> Iterator for Wrapper<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        self.0.next()
    }
}

pub trait Room: Iterator {
    fn reserved(&self, n: usize) -> usize {
        let mut v: Vec<Option<Self::Item>> = Vec::new();
        v.reserve(n);
        v.capacity()
    }
}

impl<I: Iterator> Room for Wrapper<I> {}

pub fn reserved() -> usize {
    Wrapper([1u64].into_iter()).reserved(3)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await import(join(dir, "lib.js"));
  expect(lib.reserved()).toBe(4);
});

// Which generic collects take their items' size, \`TSize\`: one counted by
// the program, whose room check reads it, and no other, so nothing gains a
// parameter it never reads (\`counted_by_program\`, #1436).
test("only a collect counted by the program takes its items' size", () => {
  const dir = fixture("collect-room-signatures");
  writeFileSync(join(dir, "lib.rs"), `pub fn counted<T: Copy>(x: T, n: usize) -> usize {
    (0..n).zip(0..n).map(|_| x).collect::<Vec<T>>().len()
}

pub fn of_items<T: Copy>(x: T, items: &[u8]) -> usize {
    items.iter().map(|_| x).collect::<Vec<T>>().len()
}

pub fn filtered<T: Copy>(x: T, n: usize) -> usize {
    (0..n).filter(|i| i % 2 == 0).map(|_| x).collect::<Vec<T>>().len()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function counted(x, n, TSize) {");
  expect(js).toContain("$roomFor(");
  expect(js).toContain("export function ofItems(x, items) {");
  expect(js).toContain("export function filtered(x, n) {");
});

// A collect counted by the program checks its room only where its count may
// pass the limit: of literal bounds, or a literal \`take(n)\`, none, its
// collect as it reads, \`[...a, ...b]\`; of a count it reads, \`$roomFor(n,
// size)\` first, a chain's the sum of its halves' (#1441).
test("a collect whose count surely fits checks no room", () => {
  const dir = fixture("collect-room-surely-fits");
  writeFileSync(join(dir, "lib.rs"), `pub fn literal() -> Vec<usize> {
    (0..2).chain(5..7).collect()
}

pub fn chained(n: usize) -> usize {
    (0..n).chain(0..2).map(|_| [0u64; 4]).collect::<Vec<_>>().len()
}

pub fn taken(n: usize) -> usize {
    (0..n).zip(0..8).take(3).map(|_| [0u64; 4]).collect::<Vec<_>>().len()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  const body = (name: string) => js.slice(js.indexOf(`export function ${name}(`), js.indexOf("\n}\n", js.indexOf(`export function ${name}(`)));
  expect(body("literal")).toContain("return [...$lazyRange(0, 2), ...$lazyRange(5, 7)];");
  expect(body("chained")).toContain("$roomFor(Math.max(n - 0, 0) + 2, 32);");
  expect(body("taken")).not.toContain("$roomFor");
});
