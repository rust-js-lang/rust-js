// What the compiler writes of a closure's body: `() => f()` of one the
// program wrote as its call's value, where that call's JS gives
// `undefined`, as Rust's closure gives `()`; a block of one written as a
// statement, or of a call whose JS gives something else. Each test is a
// program of its own, compiled and run.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildNode, compiler, fixture, run } from "./support";

// ADR 0883: a closure the program wrote as its call's value, `|| f()`, is
// `() => f()` where the call's JS gives `undefined`: a function of the
// crate's that gives `()`, a closure of its own, or a binding that says so,
// a WebIDL operation of `undefined`. What JS reads of it, `f.call()`'s
// value, is `undefined`, as Rust's `()` is. Of a call whose JS gives
// something else, `Array.prototype.push`'s count, it's a block; and one
// written `|| { f(); }` is a block, as written.
test("a closure of its call's value is that value where it's undefined", async () => {
  const node = buildNode();
  const dir = fixture("arrow-values");
  writeFileSync(join(dir, "lib.rs"), `use js::function;
use std::cell::RefCell;

fn log(seen: &RefCell<Vec<u32>>, n: u32) {
    seen.borrow_mut().push(n);
}

fn read(f: &dyn Fn()) -> bool {
    function::call(f, None, ()).is_none()
}

pub fn values() -> Vec<bool> {
    let seen = RefCell::new(Vec::new());
    let crates = || log(&seen, 1);
    let statement = || {
        log(&seen, 2);
    };
    let pushed = || seen.borrow_mut().push(3);
    let web = || webapi::console::log(("logged",));
    vec![read(&crates), read(&statement), read(&pushed), read(&web), seen.borrow().len() == 3]
}
`);
  const crates = ["js", "node", "webapi"].flatMap((lib) => ["--extern", `${lib}=${join(node, `lib${lib}.rmeta`)}`]);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...crates, "-L", node]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const crates = () => log(seen, 1);");
  expect(js).toContain("const statement = () => {\n    log(seen, 2);\n  };");
  expect(js).toContain("const pushed = () => {\n");
  expect(js).toContain('const web = () => console.log("logged");');
  const logged: unknown[] = [];
  const previous = console.log;
  console.log = (...args: unknown[]) => {
    logged.push(args);
  };
  try {
    expect((await import(join(dir, "lib.js"))).values()).toEqual([true, true, true, true, true]);
  } finally {
    console.log = previous;
  }
  expect(logged).toEqual([["logged"]]);
});
