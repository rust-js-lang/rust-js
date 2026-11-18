// What an `on_load!` body declares at its top is its module's top in JS,
// beside each other body's and the module's own names (ADR 0267): each a name
// preparation spells apart from the module's top: one of a function's word,
// which its initializer may read, is given apart, not shadowing it; an
// import of its word gives way to it, as to the program's spelled names;
// and two bodies' of one word are two variables (#2070).

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { compiler, fixture, run, target } from "./support";

/** `source`, its modules `others`, compiled; its JS and the module. */
async function compiled(name: string, source: string, others: Record<string, string> = {}) {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), source);
  for (const [file, text] of Object.entries(others)) writeFileSync(join(dir, file), text);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), load: () => import(join(dir, "lib.js")) };
}

const seenAll = async (load: () => Promise<any>) => {
  const seen: number[] = [];
  const globals = globalThis as typeof globalThis & { onLoadSeen?: (n: number) => void };
  globals.onLoadSeen = (n) => seen.push(n);
  try {
    const lib = await load();
    return { lib, seen };
  } finally {
    delete globals.onLoadSeen;
  }
};

test("an on_load body's local beside a function of its word, which it reads, is given apart", async () => {
  const { js, load } = await compiled(
    "on-load-beside-a-function",
    `#[cfg_attr(rust_js, rust_js::link_name = "globalThis.onLoadSeen")] fn seen(n: u32) { unreachable!() }
fn x() -> u32 {
    5
}
js::on_load! {
    let x = x() + 1;
    seen(x);
}
js::on_load! {
    let x = 2;
    seen(x);
}
pub fn ready() -> u32 {
    x()
}
`,
  );
  expect(js).toContain("function x() {");
  expect(js).not.toMatch(/\bconst x = /);
  const { lib, seen } = await seenAll(load);
  expect(seen).toEqual([6, 2]);
  expect(lib.ready()).toBe(5);
});

test("an on_load body's local beside an import of its word, which it reads, keeps it, the import given way", async () => {
  const { js, load } = await compiled(
    "on-load-beside-an-import",
    `#[cfg_attr(rust_js, rust_js::link_name = "globalThis.onLoadSeen")] fn seen(n: u32) { unreachable!() }
mod other;
use other::x;
js::on_load! {
    let x = x() + 1;
    seen(x);
}
pub fn ready() -> u32 {
    x()
}
`,
    { "other.rs": "pub fn x() -> u32 {\n    7\n}\n" },
  );
  // The program's name outranks an import, which gives way, as it does to
  // one the program spelled (`import_names`).
  expect(js).toContain('import { x as x$1 } from "./other.js";');
  expect(js).toContain("const x = (x$1() + 1) >>> 0;");
  const { lib, seen } = await seenAll(load);
  expect(seen).toEqual([8]);
  expect(lib.ready()).toBe(7);
});
