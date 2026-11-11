// What the compiler writes of a `Vec`'s capacity in a crate that reads one,
// where std's growth of it isn't kept (#1180): `capacity()` throws a refusal
// naming #1180, never another number. Each test is a program of its own,
// compiled and run.

import { expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
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
