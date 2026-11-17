// A library's private state that a crate using it reads where it compiles a
// library's function itself: a `static` or a `thread_local!` key is state,
// which a copy would make a second of, so the library's JS exports it by an
// internal name, `export { LIMIT as $$LIMIT }`, which its manifest lists and
// its declarations don't, and the crate imports it (#2017, ADR 0914,
// amended). Each test compiles a library and a crate that uses it, and runs
// what it wrote.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { compiled, js } from "./crates";
import { buildReact, compiler, fixture, run } from "./support";

beforeAll(buildReact, 600_000);

// The crate compiles `Holder::copies` itself, as it copies what the crate
// changes in place (`Copying`), and each library function that calls in
// turn: a private `static`, read through a private function (#2017's own);
// a thread-local made where it's first used, called, which reads what
// the crate set first; a thread-local of a
// value, its module's variable, which its library's `pub fn` sets and the
// crate reads as set; and a private module's `static`, two calls down.
test("a library's private state is read by the crate's compile of its functions", async () => {
  const dir = fixture("cross-crate-private-state");
  const { dep, app } = compiled(dir, `use std::cell::Cell;

#[derive(Clone, Copy, Debug)]
pub struct P(pub u32, pub u32);

pub struct Holder {
    items: Vec<P>,
}

static LIMIT: usize = 2;

fn limit() -> usize {
    LIMIT
}

thread_local! {
    static SKIPPED: Vec<usize> = vec![1];
    static TAKEN: Cell<usize> = Cell::new(3);
    static BASE: Vec<usize> = vec![TAKEN.get()];
}

pub fn take_at_most(n: usize) {
    TAKEN.set(n);
}

mod config {
    static STEP: usize = 2;

    fn step() -> usize {
        STEP
    }

    pub(crate) fn stepped() -> usize {
        step()
    }
}

pub mod shapes {
    static SIDES: usize = 4;

    pub fn sides() -> usize {
        SIDES
    }
}

pub use shapes::*;

impl Holder {
    pub fn new(items: Vec<P>) -> Holder {
        Holder { items }
    }

    pub fn copies(&self) -> std::iter::Take<std::iter::Copied<std::slice::Iter<'_, P>>> {
        self.items.iter().copied().take(limit())
    }

    pub fn skipped(&self) -> std::iter::Skip<std::iter::Copied<std::slice::Iter<'_, P>>> {
        self.items.iter().copied().skip(SKIPPED.with(|s| s[0]))
    }

    pub fn taken(&self) -> std::iter::Take<std::iter::Copied<std::slice::Iter<'_, P>>> {
        self.items.iter().copied().take(TAKEN.get())
    }

    pub fn based(&self) -> std::iter::Take<std::iter::Copied<std::slice::Iter<'_, P>>> {
        self.items.iter().copied().take(BASE.with(|b| b[0]))
    }

    pub fn stepped(&self) -> std::iter::StepBy<std::iter::Copied<std::slice::Iter<'_, P>>> {
        self.items.iter().copied().step_by(config::stepped())
    }
}
`, `static LIMIT: usize = 5;

pub fn changed() -> Vec<usize> {
    let h = dep::Holder::new(vec![dep::P(1, 10), dep::P(2, 20), dep::P(3, 30)]);
    let mut x = h.copies().next().unwrap();
    x.1 = 0;
    let mut y = h.skipped().next().unwrap();
    y.0 = 0;
    dep::take_at_most(1);
    let mut z = h.taken().last().unwrap();
    z.1 = 7;
    let mut v = h.based().last().unwrap();
    v.0 = 5;
    let mut w = h.stepped().last().unwrap();
    w.0 = 9;
    vec![
        x.1 as usize,
        h.copies().count(),
        y.0 as usize,
        h.skipped().count(),
        z.1 as usize,
        h.taken().count(),
        v.0 as usize,
        h.based().count(),
        w.0 as usize,
        h.stepped().count(),
        LIMIT,
        dep::sides(),
    ]
}
`);
  // Each exported by an internal name its declarations don't have.
  expect(dep).toContain("export { base as $$base, taken as $$taken, skipped as $$skipped, LIMIT as $$LIMIT };");
  const out = join(dir, "declared");
  run([compiler, "--declarations", join(dir, "dep.rs"), "-o", join(out, "lib.js"), "--library", "--manifest",
    join(out, "lib.manifest.json"), "--", "--crate-name", "dep", `--emit=metadata=${join(out, "libdep.rmeta")}`, ...js]);
  expect(readFileSync(join(out, "lib.js"), "utf8")).toContain("$$LIMIT");
  const declared = readFileSync(join(out, "lib.d.ts"), "utf8");
  expect(declared).toContain("Holder");
  expect(declared).not.toContain("$$");
  expect(app).toContain("$$LIMIT");
  // Imported by its internal name, which no name of the crate's own is, not
  // the library's word, `$$LIMIT as LIMIT`, which each of its modules would
  // reserve, giving a local of the word another: its own `LIMIT` keeps its
  // name.
  expect(app).not.toMatch(/\$\$\w+ as /);
  expect(app).toContain("const LIMIT = 5;");
  // A glob's re-export of a module that exports its state by an internal
  // name is still `export *`: its file's exports leave those out.
  expect(dep).toContain('export * from "./shapes.js";');
  const { changed } = await import(join(dir, "app", "lib.js"));
  expect(changed()).toEqual([0, 2, 0, 2, 7, 1, 5, 1, 9, 2, 5, 4]);
});
