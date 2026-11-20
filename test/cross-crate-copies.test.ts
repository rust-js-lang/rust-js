// What a crate compiles of a library's functions where they copy what it
// changes: a library's JS copies only what its own crate changes, so the
// crate that uses it compiles its own of each that copies what this crate
// changes in place (ADR 0914, amended). Each test compiles a library and a
// crate that uses it, and runs what it wrote.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { chain, compiled, pairs } from "./crates";
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

// A library's generic functions each lowered as two copies, each declaring
// the engine's drop as a name of its own, which neither uses: each copy
// leaves its own out, not the last one's alone, and the two are merged
// (ADR 1150).
test("a library's copies each leave out the drops they don't use", () => {
  const out = chain(pairs, "library_copies_unused_drops_each_lowering", ["dep", "app"]);
  const dep = readFileSync(join(out, "dep", "lib.js"), "utf8");
  // One `encode`, its copies merged, and none of them, nor `inner`, given
  // the engine's drop: only the impl a consumer asks for takes it.
  expect(dep).toContain("function inner(engine, input, out, EEngine) {\n");
  expect(dep).toContain("  encode(chunked, bytes, sink, EEngine, SSink) {\n");
  expect(dep.match(/\bencode\(chunked/g)).toHaveLength(1);
}, 300_000);

// A library's contract says which copies its functions leave out
// (`Library.elided`), written where a copy is decided, never by asking
// whether a type may be changed (#2204): `shown` asks that of `Plain` at
// its move into `take`, to read `p.x` before it (#2201), and leaves no copy
// of a `Plain` out, so its contract says none, and its consumer, which
// changes a `Plain`, imports it.
test("a library's contract lists no copy its function only asked about", () => {
  const dir = fixture("cross-crate-asked-only");
  const { app } = compiled(dir, `pub struct Plain {
    pub x: i32,
    pub y: i32,
}

pub fn take(p: Plain) -> i32 {
    p.y
}

pub fn shown(p: Plain) -> String {
    format!("{} {}", { p.x }, take(p))
}
`, `pub fn changed() -> String {
    let mut p = dep::Plain { x: 1, y: 2 };
    p.x += 10;
    dep::shown(p)
}
`);
  const manifest = JSON.parse(readFileSync(join(dir, "dep", "lib.manifest.json"), "utf8"));
  expect(manifest.library.elided ?? []).toEqual([]);
  expect(app).toContain("shown(p)");
  expect(app).not.toContain("function shown(");
});
