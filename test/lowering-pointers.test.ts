// What the compiler writes of a raw pointer's moves (ADR 0407, amended,
// #1349–#1353), where the corpus, which compares against native Rust,
// can't hold it: a refusal the program meets as it runs. Each test is a
// program of its own, compiled and run.

import { expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

// `ptr::copy` of more than one `[T; N]` through a pointer to part of an
// array cast to an array's type, `$ptrWhole`'s, which has no index along
// an array of such arrays: refused naming #1353, where it threw a
// `TypeError`. One, which overlaps its source, is copied as Rust's is.
test("ptr::copy of more than one [T; N] through a pointer to part of an array is refused", async () => {
  const dir = fixture("pointer-whole-copies");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::ptr;

pub fn two() -> Vec<u32> {
    let mut k: [u32; 6] = [0, 1, 2, 3, 4, 5];
    unsafe {
        let p = k.as_mut_ptr();
        ptr::copy(p.cast::<[u32; 2]>(), p.add(1).cast::<[u32; 2]>(), 2);
    }
    k.to_vec()
}

pub fn one() -> Vec<u32> {
    let mut k: [u32; 6] = [0, 1, 2, 3, 4, 5];
    unsafe {
        let p = k.as_mut_ptr();
        ptr::copy(p.add(1).cast::<[u32; 3]>(), p.cast::<[u32; 3]>(), 1);
    }
    k.to_vec()
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await import(join(dir, "lib.js"));
  expect(() => lib.two()).toThrow(
    "rust-js does not support copying more than one value through a pointer to part of an array cast to an array's type, which reads memory by where it is (#1353)",
  );
  // Native Rust: [1, 2, 3, 3, 4, 5].
  expect(lib.one()).toEqual([1, 2, 3, 3, 4, 5]);
});
