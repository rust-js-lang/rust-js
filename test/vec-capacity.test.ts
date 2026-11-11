// What the compiler writes of a `Vec`'s capacity in a crate that reads one,
// where std's growth of it isn't kept (#1180): `capacity()` throws a refusal
// naming #1180, never another number. Each test is a program of its own,
// compiled and run.

import { expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

// A `Vec`'s capacity std's growth of it wasn't kept of, in a crate that
// reads one: one std may collect in place, whose capacity is its source's,
// where its type is known or generic code collects it into the `Vec` its
// caller gives as `B: FromIterator` (#1263), and a string's bytes, whose capacity is the string's. `capacity()` of it
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
  for (const f of [lib.inPlace, lib.genericInPlace, lib.bytes, lib.pushed, lib.caught]) {
    expect(() => f()).toThrow("rust-js does not support `capacity()` of a `Vec` whose growth it didn't keep");
  }
  expect([lib.fitted(), lib.known()]).toEqual([5, 8]);
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
