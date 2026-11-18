// Iterators given across a library's boundary: a crate's iterator, or an
// adapter of one, given to a library's generic function, whose body the
// crate can't see, is given what that function's bounds say it may step
// (ADR 0545). Each test compiles a library and a crate that uses it, and
// runs what it wrote.

import { beforeAll, expect, test } from "bun:test";
import { join } from "node:path";
import { compiled } from "./crates";
import { buildReact, fixture } from "./support";

beforeAll(buildReact, 600_000);

const slice = `pub struct Slice<'a>(pub &'a [u32]);

impl<'a> Iterator for Slice<'a> {
    type Item = &'a u32;

    fn next(&mut self) -> Option<&'a u32> {
        let (first, rest) = self.0.split_first()?;
        self.0 = rest;
        Some(first)
    }
}

impl<'a> DoubleEndedIterator for Slice<'a> {
    fn next_back(&mut self) -> Option<&'a u32> {
        let (last, rest) = self.0.split_last()?;
        self.0 = rest;
        Some(last)
    }
}
`;

// A copy of a library's own iterator given to its `I: DoubleEndedIterator`
// function, which steps it from its back: the copy's maker gives its source
// its own `next_back`, as a map's does, where it gave its `next` alone and
// the library's `rfind` threw `it.back is not a function` (#2068, found by
// the metamorphic net's `library` rewrite of associated_double_ended.rs).
test("an adapter of a library's iterator given to its double-ended function steps from its back", async () => {
  const dir = fixture("cross-crate-iterators-back");
  compiled(dir, `${slice}
pub fn last_of<I: DoubleEndedIterator>(mut it: I) -> Option<I::Item> {
    it.next_back()
}

pub fn last_even<I: DoubleEndedIterator<Item = u32>>(mut it: I) -> Option<u32> {
    it.rfind(|x| x % 2 == 0)
}

pub fn backwards<I: DoubleEndedIterator>(it: I) -> Vec<I::Item> {
    it.rev().collect()
}
`, `use dep::{Slice, backwards, last_even, last_of};

pub fn stepped() -> Vec<String> {
    let a = [2u32, 4, 5, 7];
    vec![
        format!("{:?}", last_even(Slice(&a).copied())),
        format!("{:?}", last_of(Slice(&a).cloned())),
        format!("{:?}", backwards(Slice(&a).copied())),
        format!("{:?}", last_of(Slice(&a).map(|x| x + 1))),
        format!("{:?}", last_of(Slice(&a).filter(|x| **x % 2 == 0))),
        format!("{:?}", last_of(Slice(&a))),
    ]
}
`);
  const { stepped } = await import(join(dir, "app", "lib.js"));
  expect(stepped()).toEqual(["Some(4)", "Some(7)", "[7, 5, 4, 2]", "Some(8)", "Some(4)", "Some(7)"]);
});
