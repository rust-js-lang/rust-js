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

// A reader of the crate's own sees each read's size, which std's
// `read_to_end` takes of the capacity of what it reads into: of a `String`
// not provably fresh-empty, whose capacity a JS string doesn't carry, or a
// `Vec` whose capacity rust-js didn't keep, it's refused as the program
// meets it, where no compile error could be (#1297, #1261): generic code's,
// a `dyn`'s. Std's own readers, whose reads no program sees, read it.
test("a crate's reader read into what has a capacity not known is refused as it runs", async () => {
  const dir = fixture("own-reader-unknown-capacity");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::io::{self, Read};

struct Own(usize);

impl Read for Own {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = buf.len().min(self.0);
        buf[..n].fill(b'x');
        self.0 -= n;
        Ok(n)
    }
}

fn read_into<R: Read>(mut reader: R, s: &mut String) -> usize {
    reader.read_to_string(s).unwrap()
}

pub fn generic_own() -> usize {
    read_into(Own(3), &mut String::with_capacity(64))
}

pub fn generic_take_own() -> usize {
    read_into(Own(3).take(2), &mut String::with_capacity(64))
}

pub fn generic_std() -> usize {
    read_into(&b"abc"[..], &mut String::with_capacity(64))
}

pub fn dyn_own() -> usize {
    let mut own = Own(3);
    let reader: &mut dyn Read = &mut own;
    let mut v = String::from("ab").into_bytes();
    reader.read_to_end(&mut v).unwrap()
}

pub fn bytes_own() -> usize {
    let mut v = String::from("ab").into_bytes();
    Own(3).read_to_end(&mut v).unwrap()
}

pub fn bytes_std() -> usize {
    let mut v = String::from("ab").into_bytes();
    (&b"abc"[..]).take(2).read_to_end(&mut v).unwrap()
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await import(join(dir, "lib.js"));
  const refused = "rust-js does not support reading a reader of the crate's own to its end into a `Vec` whose capacity it didn't keep";
  expect(() => lib.genericOwn()).toThrow(refused);
  expect(() => lib.genericTakeOwn()).toThrow(refused);
  expect(() => lib.dynOwn()).toThrow(refused);
  expect(() => lib.bytesOwn()).toThrow(refused);
  expect([lib.genericStd(), lib.bytesStd()]).toEqual([3, 2]);
});
