// Which cells a held borrow can be of (ADR 0863, amended, issue #445): a
// cell's flow, where it's made and where it goes, and a JS collection's
// mark that it keeps what it's given only in itself (ADR 1045). Each test
// is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, target } from "./support";

beforeAll(buildReact, 600_000);

const withBindings = [
  "--",
  "--extern",
  `js=${join(target, "libjs.rmeta")}`,
  "-L",
  target,
];

// ADR 1045: the mark is a binding's that runs nothing, of a JS object.
test("a collection's mark is refused where it can't be said", () => {
  const dir = fixture("collection-mark-refused");
  writeFileSync(join(dir, "lib.rs"), `use std::marker::PhantomData;

pub struct Bag<T>(PhantomData<js::JsObject>, PhantomData<T>);

impl<T> Bag<T> {
    #[cfg_attr(rust_js, rust_js::link_name = "push")]
    #[cfg_attr(rust_js, rust_js::keeps_in_collection)]
    pub fn push(&self, value: T) {
        let _ = value;
        unreachable!()
    }
}

#[cfg_attr(rust_js, rust_js::keeps_in_collection)]
pub fn kept(n: u32) -> u32 {
    n
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.twice")]
#[cfg_attr(rust_js, rust_js::runs_nothing)]
#[cfg_attr(rust_js, rust_js::keeps_in_collection)]
pub fn twice(n: u32) -> u32 {
    let _ = n;
    unreachable!()
}
`);
  const p = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withBindings], {
    stderr: "pipe",
  });
  expect(p.exitCode).not.toBe(0);
  const stderr = p.stderr.toString();
  expect(stderr).toContain("it says `#[rust_js::runs_nothing]` too");
  expect(stderr).toContain("this one's body is the program's");
  expect(stderr).toContain("this one has neither");
});
