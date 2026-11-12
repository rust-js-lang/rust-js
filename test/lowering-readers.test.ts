// What the compiler writes of std's readers of others, `chain` and `take`
// (ADR 0425, amended, #1156), where the corpus, which compares against
// native Rust, can't hold it: a refusal the program meets as it runs. Each
// test is a program of its own, compiled and run.

import { expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

// A `Vec` a `Chain`'s or a `Take`'s std-default `read_to_end`,
// `$readerDefaults`', grows past its room has std's capacity, alloc's
// `default_read_to_end` ported (`$defaultReadToEnd`), of each source's
// `read_buf` (#1340); one it fits keeps std's, as does one a `Chain`'s own
// `read_to_end` grows, each source's reserving (#1338).
test("capacity() of a Vec a take's or a chain's std-default read_to_end grew is std's", async () => {
  const dir = fixture("reader-defaults-capacity");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::io::{self, Read};

pub fn grown_by_take() -> usize {
    let mut v = Vec::new();
    io::repeat(1).take(50).read_to_end(&mut v).unwrap();
    v.capacity()
}

pub fn grown_by_take_of_chain() -> usize {
    let mut v = Vec::with_capacity(2);
    (&b"ab"[..]).take(9).chain(&b"cd"[..]).take(5).read_to_end(&mut v).unwrap();
    v.capacity()
}

pub fn fitted() -> usize {
    let mut v = Vec::with_capacity(100);
    io::repeat(1).take(50).read_to_end(&mut v).unwrap();
    v.capacity()
}

pub fn chained() -> usize {
    let mut v = Vec::new();
    (&b"ab"[..]).chain(&b"cd\\nef"[..]).read_to_end(&mut v).unwrap();
    v.capacity()
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await import(join(dir, "lib.js"));
  // Native Rust: 128, 36, 100 and 8.
  expect([lib.grownByTake(), lib.grownByTakeOfChain(), lib.fitted(), lib.chained()]).toEqual([128, 36, 100, 8]);
});
