// What a crate compiles of a library's functions where they copy what it
// changes: a library's JS copies only what its own crate changes, so the
// crate that uses it compiles its own of each that copies what this crate
// changes in place (ADR 0914, amended). Each test compiles a library and a
// crate that uses it, and runs what it wrote.

import { beforeAll, expect, test } from "bun:test";
import { join } from "node:path";
import { compiled } from "./crates";
import { buildReact, fixture } from "./support";

beforeAll(buildReact, 600_000);

// A library's function that gives copies of what it borrows, an iterator
// whose items are copied or cloned as it's stepped, a `map` whose closure
// copies and an `Option`'s copy, of items the crate that uses it changes:
// the library's JS copies only what its own crate changes, so that crate
// compiles its own of each (ADR 0914, amended), and a copy it changes
// leaves its source as it was (#2014).
test("a library's copies of what it borrows are the crate's own where it changes them", async () => {
  const dir = fixture("cross-crate-copies");
  const { app } = compiled(dir, `#[derive(Clone, Copy, Debug)]
pub struct P(pub u32, pub u32);

pub fn copies(ps: &[P]) -> std::iter::Copied<std::slice::Iter<'_, P>> {
    ps.iter().copied()
}

pub fn clones(ps: &[P]) -> std::iter::Cloned<std::slice::Iter<'_, P>> {
    ps.iter().cloned()
}

pub fn first(ps: &[P]) -> Option<P> {
    ps.first().copied()
}
`, `pub fn changed() -> Vec<u32> {
    let ps = [dep::P(1, 10), dep::P(2, 20)];
    let mut a = dep::copies(&ps).next().unwrap();
    a.1 = 0;
    let mut b = dep::clones(&ps).last().unwrap();
    b.0 = 0;
    let mut c = dep::first(&ps).unwrap();
    c.0 = 9;
    vec![a.1, b.0, c.0, ps[0].0, ps[0].1, ps[1].0]
}
`);
  expect(app).toContain("function copies(ps) {");
  const { changed } = await import(join(dir, "app", "lib.js"));
  expect(changed()).toEqual([0, 0, 9, 1, 10, 2]);
});
