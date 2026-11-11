// What the compiler writes of a raw pointer's moves (ADR 0407, amended,
// #1349–#1353), where the corpus, which compares against native Rust,
// can't hold it: a refusal the program meets as it runs. Each test is a
// program of its own, compiled and run.

import { expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

// `ptr::copy` of more than one `[T; N]` through a pointer to part of an
// array cast to an array's type, `$ptrWhole`'s: as many items of the array
// it's part of, read before any is written (#1353), where it threw a
// `TypeError`; of an array that's part of none, past its end, refused. One,
// which overlaps its source, is copied as Rust's is.
test("ptr::copy of more than one [T; N] through a pointer to part of an array copies them", async () => {
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

pub fn past() -> Vec<u32> {
    let a: [u32; 2] = [1, 2];
    let mut b = [0u32; 4];
    unsafe {
        ptr::copy(&a as *const [u32; 2], b.as_mut_ptr().cast::<[u32; 2]>(), 2);
    }
    b.to_vec()
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
  // Native Rust: [0, 0, 1, 2, 3, 5].
  expect(lib.two()).toEqual([0, 0, 1, 2, 3, 5]);
  // Past the end of an array that's part of none, which Rust leaves
  // undefined: refused where it runs.
  expect(() => lib.past()).toThrow(
    "rust-js does not support copying more than one array through a pointer to one that isn't part of another",
  );
  // Native Rust: [1, 2, 3, 3, 4, 5].
  expect(lib.one()).toEqual([1, 2, 3, 3, 4, 5]);
});

// `CString::from_raw` of a pointer `into_raw` didn't give, past a C string's
// start, which Rust leaves undefined: refused where it runs (#1291).
test("CString::from_raw of a pointer into_raw didn't give is refused where it runs", async () => {
  const dir = fixture("c-string-from-raw-past");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::ffi::CString;

pub fn past() -> usize {
    let raw = CString::new("ab").unwrap().into_raw();
    unsafe { CString::from_raw(raw.add(1)).as_bytes().len() }
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await import(join(dir, "lib.js"));
  expect(() => lib.past()).toThrow("`CString::from_raw` of a pointer `CString::into_raw` didn't give");
});
